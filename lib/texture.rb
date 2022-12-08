require 'base64'

module Texture
  class Color
    attr_reader :red, :green, :blue

    def initialize(red, green, blue)
      @red = red
      @green = green
      @blue = blue
    end

    def self.from_hex(hex)
      # Have to use javascript regex for Opal :(
      raise "Tried to initialize color with invalid format" unless (/\A#[0-9a-zA-Z]{6}\z/).match?(hex)
      red = hex[1..2].to_i(16) / 255.0
      green = hex[3..4].to_i(16) / 255.0
      blue = hex[5..6].to_i(16) / 255.0
      Color.new(red, green, blue)
    end

    def to_s(hash: false)
      red_s = (@red * 255.0).round.to_s(16).rjust(2, "0")
      green_s = (@green * 255.0).round.to_s(16).rjust(2, "0")
      blue_s = (@blue * 255.0).round.to_s(16).rjust(2, "0")
      if hash
        return "##{red_s}#{green_s}#{blue_s}"
      else
        return "#{red_s}#{green_s}#{blue_s}"
      end
    end

    def +(other)
      Color.new(@red + other.red, @green + other.green, @blue + other.blue)
    end

    def *(other)
      Color.new(@red * other, @green * other, @blue * other)
    end
  end

  VMT = Struct.new(:text, :name)

  VTF = Base64.decode64(
    <<~VTFTMP
    VlRGAAcAAAACAAAAUAAAAAQABAABAwAAAQAAAAAAAAAAAIA/AACAPwAAgD8AAAAAAACAPwMAAAAB
    DQAAAAQEAQAAAAAAAAAAAAAAAAAAAAD//wAAAAAAAP//////////////////////////////////
    /////////////////////////////w==
    VTFTMP
  )

  def self.generate_unlit_color(color, vtf_name)
    vmt_text = <<~VMTTMP
    "UnlitGeneric"
    {
        "$basetexture" "#{vtf_name}"
        "$model" "1"
        "$color2" "[#{color.red} #{color.green} #{color.blue}]"
    }
    VMTTMP
    vmt = VMT.new(vmt_text, "botpath-#{color.to_s}")

    return [[vmt], [0]]
  end

  def self.generate_unlit_gradient(start_color, end_color, sample_count, vtf_name)
    vmts = []
    (0...sample_count).each do |i|
      t = i / (sample_count - 1).to_f
      color = start_color * (1 - t) + end_color * t
      vmt_text = <<~VMTTMP
      "UnlitGeneric"
      {
          "$basetexture" "#{vtf_name}"
          "$model" "1"
          "$color2" "[#{color.red} #{color.green} #{color.blue}]"
      }
      VMTTMP
      vmts << VMT.new(vmt_text, "botpath-#{color.to_s}")
    end

    vmt_indices = (0...sample_count).to_a + (1...(sample_count - 1)).to_a.reverse
    return [vmts, vmt_indices]
  end
end
