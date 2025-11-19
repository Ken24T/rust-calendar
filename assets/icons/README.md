# Calendar Icon

To add a custom icon to the application:

1. Create or find a calendar icon image (PNG, SVG, etc.)
2. Convert it to .ico format with multiple sizes (16x16, 32x32, 48x48, 256x256)

## Easy Options:

### Option 1: Online Converter
- Go to https://convertio.co/png-ico/ or https://www.icoconverter.com/
- Upload your calendar image
- Download the .ico file
- Save it as `calendar.ico` in this directory

### Option 2: Use GIMP (Free)
- Open your image in GIMP
- File > Export As > calendar.ico
- Select multiple sizes in the export dialog

### Option 3: Use ImageMagick
```powershell
magick convert calendar.png -define icon:auto-resize=256,48,32,16 calendar.ico
```

## Recommended Icon Style:
- Simple calendar grid design
- Clear at small sizes (16x16)
- Modern, flat design
- High contrast for visibility

Once you have `calendar.ico` in this directory, rebuild the project:
```powershell
cargo build --release
```

The new .exe will have your custom icon!
