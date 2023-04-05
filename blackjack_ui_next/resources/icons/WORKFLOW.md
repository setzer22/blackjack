# Icon workflow

## Creating icon image files
- Download, or create icon.
- Make sure it's exported as 32x32 pixels.
- Using white as the color base, set brightness to -47 in gimp (Colors > Brightness - Contrast > Brightness Slider).
- Make sure the icon's base color has 100% opacity (it's ok to have less in the borders due to anti-aliasing).

## Adding icons to the code
- Icons are registered in `icon_management.rs`, look for the list of `def_icon!` macro calls.
