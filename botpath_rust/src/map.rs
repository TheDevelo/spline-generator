use anyhow::*;
use cgmath::{Vector2, Vector3};
use cgmath::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MapVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    color: [f32; 3],
}

impl MapVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x3];
}

impl crate::Vertex for MapVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MapVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &MapVertex::ATTRIBS,
        }
    }
}

pub struct Map {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl Map {
    pub fn from_string(vmf_string: &str, device: &wgpu::Device) -> Result<Self> {
        let vmf = VMF::from_string(vmf_string)?;

        // Grab all the solids to render
        let world_solids = vmf.root.get_one("world")?.get_all("solid")?;
        let entity_solids = vmf.root.get_all("entity")?.iter().filter(|e| {
            let classname = e.get_one("classname");
            if !classname.is_ok() {
                return false;
            }
            classname.unwrap().to_str().unwrap_or("") == "func_detail"
        }).map (|e| e.get_all("solid")).collect::<Result<Vec<_>>>()?.into_iter().flatten().collect::<Vec<_>>();

        // Convert the solids into its constituant sides, filtering out any nodraw or clip brushes
        let mut sides = Vec::new();
        for solid in world_solids {
            let mut solid_sides = solid.get_all("side")?.iter().filter(is_side_visible).collect();
            sides.append(&mut solid_sides);
        };
        for solid in entity_solids {
            let mut solid_sides = solid.get_all("side")?.iter().filter(is_side_visible).collect();
            sides.append(&mut solid_sides);
        };

        // Construct our vertex and index bufferes from each side
        let mut vertices = Vec::new();
        let mut indices: Vec<u32> = Vec::new();

        for side in sides {
            let initial_index = vertices.len() as u32;

            // NOTE: We grab the vertices from vertices_plus, which is a Hammer++ exlcusive field.
            // I'll figure out calculating vertices from the planes given by normal Hammer later
            let side_vertices = side.get_one("vertices_plus")?.get_all("v")?.iter().map(|v| v.to_vertex()).collect::<Result<Vec<_>>>()?;
            ensure!(side_vertices.len() >= 3, "VMF contains face with less than 3 vertices");

            // Calculate the normal for the face
            let cb = side_vertices[2] - side_vertices[1];
            let ab = side_vertices[0] - side_vertices[1];
            let normal = cb.cross(ab).normalize();

            let material = side.get_one("material")?.to_str()?.to_uppercase();
            let color;
            if material == "TOOLS/TOOLSSKYBOX" || material == "TOOLS/TOOLSSKYBOX2D" {
                color = Vector3::new(0.0, 1.0, 1.0);
            }
            else {
                // For non-skybox faces, calculate the color of the side based on the surface normal
                color = normal / 2.0 + Vector3::new(0.5, 0.5, 0.5);
            }

            for vertex in &side_vertices {
                let uv = calculate_uvs(vertex, &normal);
                vertices.push(MapVertex {
                    position: [vertex.x, vertex.y, vertex.z],
                    tex_coords: [uv.x, uv.y],
                    color: [color.x, color.y, color.z],
                });
            }

            for i in 1..(side_vertices.len() - 1) {
                // We push the indices in this order to ensure they are CCW
                let i = i as u32;
                indices.push(initial_index);
                indices.push(initial_index + i + 1);
                indices.push(initial_index + i + 0);
            }
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Map Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Map Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Ok(Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        })
    }

    pub fn draw<'s>(&'s self, render_pass: &mut wgpu::RenderPass<'s>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

const TEXTURE_SCALE: f32 = 256.0;

// Function to calculate the UV values for a given vertex. The UV values are scaled world-space XYZ
// coordinates, with the two axes selected to minimize texture stretching. The two axes that
// minimize stretching are the two that contribute to the normal the least
fn calculate_uvs(vertex: &Vector3<f32>, normal: &Vector3<f32>) -> Vector2<f32> {
    if normal.x.abs() <= normal.z.abs() && normal.y.abs() <= normal.z.abs() {
        return Vector2::new(vertex.x / TEXTURE_SCALE, vertex.y / TEXTURE_SCALE);
    }
    else if normal.x.abs() <= normal.y.abs() && normal.z.abs() <= normal.y.abs() {
        return Vector2::new(vertex.x / TEXTURE_SCALE, vertex.z / TEXTURE_SCALE);
    }
    else {
        return Vector2::new(vertex.y / TEXTURE_SCALE, vertex.z / TEXTURE_SCALE);
    }
}

// Function to filter out sides with tools textures that aren't visible in game
fn is_side_visible(side: &&VMFEntry) -> bool {
    let material = side.get_one("material");
    if !material.is_ok() {
        // Doesn't contain a material somehow
        return false;
    }
    let material = material.unwrap().to_str();
    if !material.is_ok() {
        // Material is not a string somehow
        return false;
    }
    let material = material.unwrap().to_uppercase();

    material != "TOOLS/TOOLSNODRAW"
        && material != "TOOLS/TOOLSPLAYERCLIP"
        && material != "TOOLS/TOOLSCLIP"
        && material != "TOOLS/TOOLSTRIGGER"
        && material != "TOOLS/TOOLSHINT"
        && material != "TOOLS/TOOLSSKIP"
}

// VMF types to parse the VMF file into a traversible structure
type VMFBranch = HashMap<String, Vec<VMFEntry>>;

enum VMFEntry {
    Branch(VMFBranch),
    Leaf(String),
}

impl VMFEntry {
    fn to_vertex(&self) -> Result<Vector3<f32>> {
        if let VMFEntry::Leaf(value) = self {
            let vertex_coord_strs: Vec<&str> = value.split(" ").collect();
            if vertex_coord_strs.len() != 3 {
                bail!("VMF vertex doesn't contain 3 entries");
            }

            let x_val = vertex_coord_strs[0].parse()?;
            let y_val = vertex_coord_strs[1].parse()?;
            let z_val = vertex_coord_strs[2].parse()?;

            return Ok(Vector3::new(x_val, y_val, z_val));
        }
        else {
            bail!("Can't convert VMF branch into vertex");
        }
    }

    fn to_str(&self) -> Result<&str> {
        if let VMFEntry::Leaf(value) = self {
            return Ok(value);
        }
        else {
            bail!("Can't convert VMF branch into string");
        }
    }

    fn get_one(&self, key: &str) -> Result<&VMFEntry> {
        if let VMFEntry::Branch(branch) = self {
            let values = branch.get(key).ok_or(anyhow!("VMF branch doesn't contain specified key"))?;
            if values.len() != 1 {
                bail!("VMF branch contains more than one value");
            }

            return Ok(&values[0]);
        }
        else {
            bail!("can't call get_one on a VMF leaf");
        }
    }

    fn get_all(&self, key: &str) -> Result<&Vec<VMFEntry>> {
        if let VMFEntry::Branch(branch) = self {
            let values = branch.get(key).ok_or(anyhow!("VMF branch doesn't contain specified key"))?;
            return Ok(values);
        }
        else {
            bail!("can't call get_all on a VMF leaf");
        }
    }
}

struct VMF {
    root: VMFEntry,
}

impl VMF {
    // Parse a VMF file into a VMF struct
    fn from_string(vmf_string: &str) -> Result<Self> {
        let mut current_branch = VMFBranch::new();
        let mut tree_stack = Vec::<(VMFBranch, String)>::new(); // Holds parents of current branch all the way up the VMF tree
        let leaf_regex = Regex::new("^\"(.*)\" \"(.*)\"$").unwrap();

        // We construct our VMF line by line
        let mut vmf_lines = vmf_string.lines();
        while let Some(line) = vmf_lines.next() {
            let line = line.trim();
            // Case 1: Line closes the current branch, so traverse back up the tree and add our
            // finalized branch to its parent.
            if line == "}" {
                let new_branch = tree_stack.pop();
                if let Some(new_branch) = new_branch {
                    let mut parent = new_branch.0;
                    let child_name = new_branch.1;
                    if let Some(entries) = parent.get_mut(&child_name) {
                        entries.push(VMFEntry::Branch(current_branch));
                    }
                    else {
                        parent.insert(child_name, vec![VMFEntry::Branch(current_branch)]);
                    }
                    current_branch = parent;
                }
                else {
                    // VMF closes the root branch. Since the root branch is the whole file,
                    // it shouldn't be closed by an ending brace.
                    bail!("invalid VMF structure");
                }
            }
            // Case 2: Line specifies a leaf entry, so add to our current branch
            else if let Some(captures) = leaf_regex.captures(line) {
                let name = captures.get(1).unwrap().as_str().to_string();
                let value = captures.get(2).unwrap().as_str().to_string();

                if let Some(entries) = current_branch.get_mut(&name) {
                    entries.push(VMFEntry::Leaf(value));
                }
                else {
                    current_branch.insert(name, vec![VMFEntry::Leaf(value)]);
                }
            }
            // Case 3: Line specifies a new branch (must be nonempty), so move down the branch hierarchy
            else if line != "" {
                // The opening brace lies on the next line, so grab it to check if we actually
                // satisfy the new branch syntax. This is the last case, so OK to error
                let next_line = vmf_lines.next().unwrap_or("").trim();
                if next_line != "{" {
                    bail!("malformed VMF syntax");
                }

                tree_stack.push((current_branch, line.to_string()));
                current_branch = VMFBranch::new();
            }
        }

        // Check that our VMF actually closed every branch
        if tree_stack.len() != 0 {
            bail!("invalid VMF structure");
        }

        Ok(Self {
            root: VMFEntry::Branch(current_branch),
        })
    }
}
