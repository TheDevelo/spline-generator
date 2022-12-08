require 'base64'

require 'file_parser'
require 'model'
require 'texture'

def generate_zip(file, name, radius, color, gradient, sides, prisms)
  radius = radius.to_f
  sides = [sides.to_i, 2].max
  color = Texture::Color.from_hex color
  gradient = Texture::Color.from_hex gradient unless gradient.nil?
  prisms = [prisms.to_i, 1].max unless prisms.nil?

  verts = FileParser.parse_log(file)
  vtf = Texture::VTF
  vtf64 = Base64.encode64(vtf)
  if gradient.nil?
    vmt_spec = Texture.generate_unlit_color(color, "bp-gen/botpath")
  else
    vmt_spec = Texture.generate_unlit_gradient(color, gradient, 16, "bp-gen/botpath")
  end
  model_pairs = Model.generate_model(verts, name, radius, vmt_spec, "bp-gen", sides, prisms)

  return [model_pairs, vtf64, vmt_spec[0].map {|vmt| [vmt.text, vmt.name] }]
end
