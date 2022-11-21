require 'base64'

require 'file_parser'
require 'model'
require 'texture'

def generate_zip(file, name, radius, color, sides, prisms)
  radius = radius.to_f
  sides = [sides.to_i, 2].max
  color = Texture::Color.new color
  prisms = [prisms.to_i, 1].max if prisms != nil

  verts = FileParser.parse_log(file)
  vtf, vmt = Texture.generate_unlit_color(color, "bp-gen/botpath")
  vtf64 = Base64.encode64(vtf)
  model_pairs = Model.generate_model(verts, name, radius, "botpath-#{color.to_s}", "bp-gen", sides, prisms)

  return [model_pairs, vtf64, vmt]
end
