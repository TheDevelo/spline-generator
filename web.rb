require 'base64'

require 'model'
require 'parser'
require 'texture'

def generate_zip(file, name, size, color)
  size = size.to_f

  verts = Parser.parse_log(file)
  vtf, vmt = Texture.generate_unlit_color(color, "bp-gen/botpath")
  vtf64 = Base64.encode64(vtf)
  smd, qc = Model.generate_model(verts, name, size / 2, "botpath-#{color}", "bp-gen", 6)

  return [smd, qc, vtf64, vmt]
end
