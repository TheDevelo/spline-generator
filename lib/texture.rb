require 'base64'

module Texture
  class Color
    attr_reader :red, :green, :blue

    def initialize(hex)
      # Have to use javascript regex for Opal :(
      raise "Tried to initialize color with invalid format" unless (/\A#[0-9a-zA-Z]{6}\z/).match?(hex)
      @red = hex[1..2].to_i(16) / 255.0
      @green = hex[3..4].to_i(16) / 255.0
      @blue = hex[5..6].to_i(16) / 255.0
    end

    def to_s(hash: false)
      red_s = (@red * 255.5).to_i.to_s(16).rjust(2, "0")
      green_s = (@green * 255.5).to_i.to_s(16).rjust(2, "0")
      blue_s = (@blue * 255.5).to_i.to_s(16).rjust(2, "0")
      if hash
        return "##{red_s}#{green_s}#{blue_s}"
      else
        return "#{red_s}#{green_s}#{blue_s}"
      end
    end
  end

  def self.generate_unlit_color(color, vtf_name)
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
        "$color2" "[#{color.red} #{color.green} #{color.blue}]"
    }
    VMTTMP

    return [vtf, vmt]
  end
end
