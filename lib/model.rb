require 'matrix'

require 'vector_ext'

module Model
  def self.generate_model(verts, name, radius, vmt, material_path, sides)
    verts = prune_vertices(verts, 0)

    # Generate vertex skeleton for model shape
    norm_vec_diff = []
    (0...(verts.length - 1)).each do |i|
      norm_vec_diff[i] = (verts[i+1] - verts[i]).normalize
    end

    mitre_planes = [norm_vec_diff[0]]
    (1...(verts.length - 1)).each do |i|
      mitre_planes[i] = (norm_vec_diff[i] + norm_vec_diff[i-1]).normalize
    end
    mitre_planes[-1] = norm_vec_diff[-2]

    skeleton = []
    angle = 2.0 * Math::PI / sides
    up_vector = Vector[0.0, 0.0, 1.0].project_plane(mitre_planes[0])
    up_vector = Vector[1.0, 0.0, 0.0] if up_vector == Vector[0.0, 0.0, 0.0]
    mitre_planes.each_with_index do |p, i|
      up_vector = radius * up_vector.project_plane(p).normalize
      face = [up_vector + verts[i]]
      (1...sides).each do |n|
        face << up_vector.rotate_around(p, angle * n) + verts[i]
      end
      skeleton << face
    end

    # Generate model files
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

    # Add start cap
    front_face = skeleton[0]
    (1...(sides-1)).each do |i|
      x = front_face[0]
      y = front_face[i+1]
      z = front_face[i]

      smd += "#{vmt}.vmt\n"
      smd += "0 #{x[0]} #{x[1]} #{x[2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{y[0]} #{y[1]} #{y[2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{z[0]} #{z[1]} #{z[2]} 0.0 0.0 0.0 0.0 0.0\n"
    end

    (0...(skeleton.length - 1)).each do |i|
      back_face = skeleton[i]
      front_face = skeleton[i+1]
      (0...sides).each do |n|
        x = back_face[n]
        y = back_face[(n + 1) % sides]
        z = front_face[n]
        w = front_face[(n + 1) % sides]

        smd += "#{vmt}.vmt\n"
        smd += "0 #{x[0]} #{x[1]} #{x[2]} 0.0 0.0 0.0 0.0 0.0\n"
        smd += "0 #{y[0]} #{y[1]} #{y[2]} 0.0 0.0 0.0 0.0 0.0\n"
        smd += "0 #{z[0]} #{z[1]} #{z[2]} 0.0 0.0 0.0 0.0 0.0\n"

        smd += "#{vmt}.vmt\n"
        smd += "0 #{w[0]} #{w[1]} #{w[2]} 0.0 0.0 0.0 0.0 0.0\n"
        smd += "0 #{z[0]} #{z[1]} #{z[2]} 0.0 0.0 0.0 0.0 0.0\n"
        smd += "0 #{y[0]} #{y[1]} #{y[2]} 0.0 0.0 0.0 0.0 0.0\n"
      end
    end

    # Add end cap
    back_face = skeleton[-1]
    (1...(sides-1)).each do |i|
      x = back_face[0]
      y = back_face[i]
      z = back_face[i+1]

      smd += "#{vmt}.vmt\n"
      smd += "0 #{x[0]} #{x[1]} #{x[2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{y[0]} #{y[1]} #{y[2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{z[0]} #{z[1]} #{z[2]} 0.0 0.0 0.0 0.0 0.0\n"
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
