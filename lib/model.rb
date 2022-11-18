module Model
  def self.generate_model(verts, name, size, vmt, material_path)
    verts = prune_vertices(verts, 0.0)

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

  # Angle in radians
  def self.prune_vertices(verts, angle)
    # Dot product of normalized vectors is acos(angle), so threshold = cos(angle)
    threshold = Math.cos(angle)

    # Pre-prune identical vertices, as we can't use diff of identical vertices
    i = 0
    while i < verts.length - 1
      if verts[i] == verts[i+1]
        verts.delete_at(i+1)
      else
        i += 1
      end
    end

    # Can't prune with only two or less vertices
    return verts if verts.length <= 2

    # Generate dot products between each consecutive pair of vertices
    norm_vec_diff = []
    (0...(verts.length - 1)).each do |i|
      norm_vec_diff[i] = (verts[i+1] - verts[i]).normalize
    end
    vert_prods = []
    (0...(norm_vec_diff.length - 1)).each do |i|
      vert_prods[i] = norm_vec_diff[i].dot norm_vec_diff[i+1]
    end

    prod, i = vert_prods.each_with_index.max
    while prod != nil and prod >= threshold
      # Remove middle vertex and recalculate affected products
      verts.delete_at(i + 1)
      norm_vec_diff.delete_at(i + 1)
      norm_vec_diff[i] = (verts[i+1] - verts[i]).normalize
      vert_prods.delete_at(i)
      # Update both unless deleted vertex next to one of the ends
      vert_prods[i] = norm_vec_diff[i].dot norm_vec_diff[i+1] unless i == vert_prods.length
      vert_prods[i-1] = norm_vec_diff[i-1].dot norm_vec_diff[i] unless i == 0

      prod, i = vert_prods.each_with_index.max
    end

    return verts
  end
end
