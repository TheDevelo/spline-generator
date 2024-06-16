use super::Spline;

use anyhow::*;
use cgmath::Point3;
use egui::{Color32, Rgba};
use indoc::{formatdoc, indoc};
use std::cell::{RefCell, Ref};
use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub fn construct_zip(splines: &[RefCell<Spline>]) -> Result<Vec<u8>> {
    // Construct the buffer we will write our Zip file to
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(Cursor::new(&mut zip_buffer));
    let options = SimpleFileOptions::default();

    // Construct the model files for each spline
    for (i, spline) in splines.iter().enumerate() {
        let spline = spline.borrow();
        if spline.data.bundle {
            continue;
        }

        // Construct the SMD file
        zip.start_file(format!("spline-{i}.smd"), options)?;
        smd_from_spline(&spline, &mut zip)?;

        // Construct the QC file
        let origin;
        if spline.data.points.len() == 0 {
            origin = Point3::new(0.0, 0.0, 0.0);
        }
        else {
            origin = spline.data.points[0].position;
        }
        zip.start_file(format!("spline-{i}.qc"), options)?;
        // We negate the origin to offset it to the first vertex
        zip.write_all(&formatdoc! {"
            $staticprop
            $modelname \"{}\"
            $origin {} {} {}
            $scale \"1.0\"
            $body \"Body\" \"spline-{i}\"
            $cdmaterials \"spline-gen\"
            $sequence idle \"spline-{i}\"
            $surfaceprop \"default\"
            $mostlyopaque
        ", spline.data.name, -origin.x, -origin.y, -origin.z}.into_bytes())?;
    }

    // Construct the required VTF/VMT files
    zip.add_directory("materials/spline-gen", options)?;
    zip.start_file("materials/spline-gen/spline.vtf", options)?;
    zip.write_all(include_bytes!("spline.vtf"))?;

    zip.start_file("materials/spline-gen/spline.vmt", options)?;
    zip.write_all(indoc! {b"
        \"UnlitGeneric\"
        {
            \"$basetexture\" \"spline-gen/spline\"
            \"$model\" \"1\"
        }
    "})?;

    zip.start_file("materials/spline-gen/spline-transparent.vmt", options)?;
    zip.write_all(indoc! {b"
        \"UnlitGeneric\"
        {
            \"$basetexture\" \"spline-gen/spline\"
            \"$model\" \"1\"
            \"$translucent\" \"1\"
        }
    "})?;

    zip.finish().unwrap();
    return Ok(zip_buffer);
}

fn smd_from_spline(spline: &Ref<Spline>, zip: &mut dyn Write) -> Result<()> {
    zip.write_all(indoc! {b"
        version 1
        nodes
        0 \"static_prop\" -1
        end
        skeleton
        time 0
        0 0.000000 0.000000 0.000000 0.000000 0.000000 0.000000
        end
        triangles
    "})?;

    for triangles in spline.indices.chunks(3) {
        let v0 = spline.vertices[triangles[0] as usize];
        let v1 = spline.vertices[triangles[1] as usize];
        let v2 = spline.vertices[triangles[2] as usize];
        let [v0x, v0y, v0z] = v0.position;
        let [v1x, v1y, v1z] = v1.position;
        let [v2x, v2y, v2z] = v2.position;
        let [n0x, n0y, n0z] = v0.normal;
        let [n1x, n1y, n1z] = v1.normal;
        let [n2x, n2y, n2z] = v2.normal;

        // Determine the color of the triangle
        // We use the color of the majority t-value
        let full_t;
        if v0.t_value == v1.t_value || v0.t_value == v2.t_value {
            full_t = v0.t_value;
        }
        else {
            full_t = v1.t_value;
        }
        let t = full_t - full_t.floor();

        let color0 = Rgba::from(spline.data.points[full_t.floor() as usize].color);
        let color1 = Rgba::from(spline.data.points[std::cmp::min(full_t.ceil() as usize, spline.data.points.len() - 1)].color);
        let (r0, g0, b0, a0) = color0.to_tuple();
        let (r1, g1, b1, a1) = color1.to_tuple();
        let (rt, gt, bt, at) = (r0 * (1.0 - t) + r1 * t, g0 * (1.0 - t) + g1 * t, b0 * (1.0 - t) + b1 * t, a0 * (1.0 - t) + a1 * t);
        let color = Color32::from(Rgba::from_rgba_premultiplied(rt, gt, bt, at));

        // Quantize the color to 32-steps and turn into a UV value
        let [mut qr, mut qg, mut qb, mut qa] = color.to_srgba_unmultiplied();
        qr /= 8;
        qg /= 8;
        qb /= 8;
        qa /= 8;

        let u = ((qr as f32 + qg as f32 * 32.0) + 0.5) / 1024.0;
        let v = 1.0 - ((qb as f32 + qa as f32 * 32.0) + 0.5) / 1024.0;

        // Write the triangle to the VMT
        // NOTE: X/Y is usually east/north, but is north/west in SMD
        // Additionally, SMD has normals point inwards instead of outwards
        let vmt_name;
        if qa == 31 {
            vmt_name = "spline.vmt";
        }
        else {
            vmt_name = "spline-transparent.vmt";
        }
        zip.write_all(&formatdoc! {"
            {vmt_name}
            0 {} {} {} {} {} {} {u} {v}
            0 {} {} {} {} {} {} {u} {v}
            0 {} {} {} {} {} {} {u} {v}
        ",
        v0y, -v0x, v0z, -n0y, n0x, -n0z,
        v1y, -v1x, v1z, -n1y, n1x, -n1z,
        v2y, -v2x, v2z, -n2y, n2x, -n2z}.into_bytes())?;
    }

    zip.write_all(b"end")?;

    Ok(())
}
