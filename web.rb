require 'base64'

require 'model'
require 'texture'

def generate_zip(file, name, size, color)
  size = size.to_f

  vtf, vmt = Texture.generate_unlit_color(color, "bp-gen/botpath")
  vtf64 = Base64.encode64(vtf)
  smd, qc = Model.generate_model(file, name, size, "botpath-#{color}", "bp-gen")

  return [smd, qc, vtf64, vmt]
end
