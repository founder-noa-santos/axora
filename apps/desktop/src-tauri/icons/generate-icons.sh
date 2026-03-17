#!/bin/bash
# Generate placeholder icons using ImageMagick

# Create 32x32 PNG
convert -size 32x32 xc:'#8B5CF6' -fill white -gravity center \
  -pointsize 16 -annotate 0 "AX" 32x32.png

# Create 128x128 PNG
convert -size 128x128 xc:'#8B5CF6' -fill white -gravity center \
  -pointsize 48 -annotate 0 "AXORA" 128x128.png

# Create 128x128@2x PNG
cp 128x128.png 128x128@2x.png

# Create ICNS (macOS)
convert -size 512x512 xc:'#8B5CF6' -fill white -gravity center \
  -pointsize 120 -annotate 0 "AX" icon.png
png2icns icon.icns icon.png
rm icon.png

# Create ICO (Windows)
convert -size 256x256 xc:'#8B5CF6' -fill white -gravity center \
  -pointsize 64 -annotate 0 "AX" icon.ico

echo "Icons generated!"
