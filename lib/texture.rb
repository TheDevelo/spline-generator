require 'base64'

module Texture
  def self.generate_unlit_color(color, vtf_name)
    color_r = color[0..1].to_i(16) / 255.0
    color_g = color[2..3].to_i(16) / 255.0
    color_b = color[4..5].to_i(16) / 255.0

    vtf = Base64.decode64(
      <<~VTFTMP
      VlRGAAcAAAACAAAAUAAAAAQABAABAwAAAQAAAAAAAAAAAIA/AACAPwAAgD8AAAAAAACAPwMAAAAB
      DQAAAAQEAQAAAAAAAAAAAAAAAAAAAAD//wAAAAAAAP//////////////////////////////////
      /////////////////////////////w==
      VTFTMP
    )

    vmt = <<~VMTTMP
    "UnlitGeneric"
    {
        "$basetexture" "#{vtf_name}"
        "$model" "1"
        "$color2" "[#{color_r} #{color_g} #{color_b}]"
    }
    VMTTMP

    return [vtf, vmt]
  end
end
