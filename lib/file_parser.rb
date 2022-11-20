require 'matrix'

module FileParser
  def self.parse_log(log)
    fp_regex = /[-+]?(?:[0-9]*\.[0-9]+|[0-9]+)/
    # getpos can sometimes return setang a b c\n setpos x y z; instead of normal
    # setpos x y z;setang a b c\n. So search for any instances and fix before parsing.
    # NOTE: Doesn't fix consecutive instances. Would have to loop over and change a line
    # before matching the next, but this bug is rare enough to not matter.
    log = log.gsub(/^setang (#{fp_regex}) (#{fp_regex}) (#{fp_regex})\nsetpos (#{fp_regex}) (#{fp_regex}) (#{fp_regex});/m,
                   "setpos \\4 \\5 \\6;setang \\1 \\2 \\3\n")

    # Parse vertices from log
    verts = []
    log.each_line do |line|
      elems = line.split(';').map {|s| s.split(' ')}

      # Check if line matches format setpos x y z;setang a b c
      if elems.length != 2 or elems[0].length != 4 or elems[1].length != 4 or
         elems[0][0] != "setpos" or elems[1][0] != "setang"
        next
      else
        float_regex = /^#{fp_regex}$/
        malformed = false
        (1..3).each do |i|
          malformed = true unless float_regex.match?(elems[0][i]) and float_regex.match?(elems[1][i])
        end
        next if malformed
      end

      vec = Vector.elements(elems[0][1..3].map {|s| s.to_f })
      verts << vec
    end

    # Shift all vertices so that the first is the origin
    if verts.length != 0
      first = verts[0]
      verts.each_index {|i| verts[i] -= first }
    end

    # Studiomdl vertices are rotated compared to in game or in Hammer, so rotate
    # each vertex 90 degrees counter-clockwise.
    verts.each do |v|
      x = v[0]
      y = v[1]
      v[0] = y
      v[1] = -x
    end

    return verts
  end
end
