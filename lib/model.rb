require 'matrix'

module Model
  def self.generate_model(file, name, size, vmt, material_path)
    positions = []
    start = nil
    file.each_line do |line|
      coms = line.split(';').map {|s| s.split(' ')}
      if coms.length != 2 or coms[0].length != 4 or coms[0][0] != "setpos"
        next
      end
      nums = coms[0][1..3].map {|s| s.to_f }
      vec = Vector.elements(nums)
      start = vec if start == nil
      positions << vec - start
    end

    positions.each do |v|
      x = v[0]
      y = v[1]
      v[0] = y
      v[1] = -x
    end

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

    (0...(positions.length - 1)).each do |i|
      smd += "#{vmt}.vmt\n"
      smd += "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]} 0.0 0.0 0.0 0.0 0.0\n"

      smd += "#{vmt}.vmt\n"
      smd += "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]} 0.0 0.0 0.0 0.0 0.0\n"

      smd += "#{vmt}.vmt\n"
      smd += "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2] + size} 0.0 0.0 0.0 0.0 0.0\n"

      smd += "#{vmt}.vmt\n"
      smd += "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]} 0.0 0.0 0.0 0.0 0.0\n"
      smd += "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2] + size} 0.0 0.0 0.0 0.0 0.0\n"
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
