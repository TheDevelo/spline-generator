module Model
  def self.generate_model(verts, name, size, vmt, material_path)
    smd = <<~SMDTMP
    version 1
    nodes
    0 "static_prop" -1
    end
    skeleton
    time 0
    0 0.000000 0.000000 0.000000 0.000000 0.000000 0.000000
    end
    triangles
    SMDTMP

    (0...(verts.length - 1)).each do |i|
      smd += "#{vmt}.vmt\n"
      smd += "0 #{verts[i][0]} #{verts[i][1]} #{verts[i][2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{verts[i][0]} #{verts[i][1]} #{verts[i][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{verts[i+1][0]} #{verts[i+1][1]} #{verts[i+1][2]} 0.0 0.0 0.0 0.0 0.0\n"

      smd += "#{vmt}.vmt\n"
      smd += "0 #{verts[i][0]} #{verts[i][1]} #{verts[i][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{verts[i][0]} #{verts[i][1]} #{verts[i][2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{verts[i+1][0]} #{verts[i+1][1]} #{verts[i+1][2]} 0.0 0.0 0.0 0.0 0.0\n"

      smd += "#{vmt}.vmt\n"
      smd += "0 #{verts[i+1][0]} #{verts[i+1][1]} #{verts[i+1][2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{verts[i+1][0]} #{verts[i+1][1]} #{verts[i+1][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{verts[i][0]} #{verts[i][1]} #{verts[i][2] + size} 0.0 0.0 0.0 0.0 0.0\n"

      smd += "#{vmt}.vmt\n"
      smd += "0 #{verts[i+1][0]} #{verts[i+1][1]} #{verts[i+1][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{verts[i+1][0]} #{verts[i+1][1]} #{verts[i+1][2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{verts[i][0]} #{verts[i][1]} #{verts[i][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
    end

    smd += "end"

    qc = <<~QCTMP
    $staticprop
    $modelname "#{name}"
    $scale "1.000000"
    $body "Body" "botpath_ref"
    $cdmaterials "#{material_path}"
    $sequence idle "botpath_ref"
    $surfaceprop "default"
    $opaque
    QCTMP

    return [smd, qc]
  end
end
