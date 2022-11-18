require 'model'
require 'parser'
require 'texture'

if ARGV.length != 1
  puts "error: pass in 1 file"
  return
end

if not File.file?(ARGV[0])
  puts "error: pass in valid file"
  return
end

verts = Parser.parse_log(File.read(ARGV[0]))
vtf, vmt = Texture.generate_unlit_color("ffffff", "bp-gen/botpath")
smd, qc = Model.generate_model(verts, "bp-gen/botpath", 16.0, "botpath-ffffff", "bp-gen")

File.write("botpath.qc", qc)
File.write("botpath_ref.smd", smd)
Dir.mkdir("materials/") unless Dir.exist?("materials/")
Dir.mkdir("materials/bp-gen/") unless Dir.exist?("materials/bp-gen/")
File.write("materials/bp-gen/botpath.vtf", vtf)
File.write("materials/bp-gen/botpath-ffffff.vmt", vmt)
