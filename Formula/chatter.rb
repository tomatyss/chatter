class Chatter < Formula
  desc "Terminal-based chat interface for Google's Gemini AI"
  homepage "https://github.com/tomatyss/chatter"
  url "https://github.com/tomatyss/chatter/archive/v0.1.0.tar.gz"
  sha256 "YOUR_SHA256_HERE"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Test that the binary exists and shows help
    assert_match "chatter", shell_output("#{bin}/chatter --help")
  end

  def caveats
    <<~EOS
      To use chatter, you need to set up your Gemini API key:

      1. Get your API key from: https://aistudio.google.com/app/apikey
      2. Set it up with: chatter config set-api-key
      3. Or export it: export GEMINI_API_KEY="your-api-key"

      Then start chatting with: chatter
    EOS
  end
end
