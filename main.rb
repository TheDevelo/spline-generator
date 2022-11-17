require 'matrix'

if ARGV.length != 1
  puts "error: pass in 1 file"
end

if not File.file?(ARGV[0])
  puts "error: pass in valid file"
end

positions = []
start = nil
File.readlines(ARGV[0]).each do |line|
  coms = line.split(';').map {|s| s.split(' ')}
  if coms.length != 2 or coms[0].length != 4 or coms[0][0] != "setpos"
    next
  end
  nums = coms[0][1..3].map {|s| s.to_f }
  vec = Vector.elements(nums)
  start = vec if start == nil
  positions << vec - start
end

smd_begin =  <<SMDTMP
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
puts smd_begin

positions.each do |v|
  x = v[0]
  y = v[1]
  v[0] = y
  v[1] = -x
end

(0...(positions.length - 1)).each do |i|
  puts "botpath.vmt"
  puts "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2]} 0.0 0.0 0.0 0.0 0.0"
  puts "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2]+16.0} 0.0 0.0 0.0 0.0 0.0"
  puts "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]} 0.0 0.0 0.0 0.0 0.0"

  puts "botpath.vmt"
  puts "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2]+16.0} 0.0 0.0 0.0 0.0 0.0"
  puts "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2]} 0.0 0.0 0.0 0.0 0.0"
  puts "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]} 0.0 0.0 0.0 0.0 0.0"

  puts "botpath.vmt"
  puts "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]} 0.0 0.0 0.0 0.0 0.0"
  puts "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]+16.0} 0.0 0.0 0.0 0.0 0.0"
  puts "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2]+16.0} 0.0 0.0 0.0 0.0 0.0"

  puts "botpath.vmt"
  puts "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]+16.0} 0.0 0.0 0.0 0.0 0.0"
  puts "0 #{positions[i+1][0]} #{positions[i+1][1]} #{positions[i+1][2]} 0.0 0.0 0.0 0.0 0.0"
  puts "0 #{positions[i][0]} #{positions[i][1]} #{positions[i][2]+16.0} 0.0 0.0 0.0 0.0 0.0"
end

puts "end"
