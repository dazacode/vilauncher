
import sys
from PIL import Image, ImageFilter, ImageOps, ImageChops, ImageEnhance
src, out = sys.argv[1:3]
L, T, B = 147, 452, 108
R = Image.open(src).convert('RGB')
W0, H0 = R.size

def hgrad(w, h, a, b):  # left->right from a to b (0..255)
    g = Image.linear_gradient('L').rotate(90, expand=True)  # 0 at left -> 255 at right
    g = g.resize((w, h))
    return g.point(lambda v: int(a + (b - a) * v / 255))

def vgrad(w, h, a, b):  # top->bottom
    g = Image.linear_gradient('L').resize((w, h))
    return g.point(lambda v: int(a + (b - a) * v / 255))

# 1. left: mirrored edge, blurred, then a soft haze into the art
W = L + W0
wide = Image.new('RGB', (W, H0))
mir = ImageOps.mirror(R.crop((0, 0, L, H0)))
wide.paste(mir, (0, 0)); wide.paste(R, (L, 0))
soft = wide.filter(ImageFilter.GaussianBlur(40))
m = Image.new('L', (W, H0), 0)
m.paste(255, (0, 0, L, H0))
m.paste(hgrad(170, H0, 255, 0), (L, 0))
wide = Image.composite(soft, wide, m)

# 2. bottom: mirrored edge, blurred
H1 = H0 + B
tall = Image.new('RGB', (W, H1))
tall.paste(wide, (0, 0))
tall.paste(ImageOps.flip(wide.crop((0, H0 - B, W, H0))), (0, H0))
soft = tall.filter(ImageFilter.GaussianBlur(9))
m = Image.new('L', (W, H1), 0)
m.paste(vgrad(W, 70, 0, 255), (0, H0 - 70))
m.paste(255, (0, H0, W, H1))
tall = Image.composite(soft, tall, m)

# 3. top: the sky's top rows, smoothed, carried up and darkened toward the top
H = T + H1
top_row = tall.crop((0, 0, W, 10)).resize((W, 1), Image.BOX)
strip = top_row.resize((W, 40)).filter(ImageFilter.GaussianBlur(60)).resize((W, 1), Image.BOX)
sky = strip.resize((W, T + 60), Image.BILINEAR)
dark = Image.new('RGB', sky.size, (26, 20, 44))
sky = Image.composite(dark, sky, vgrad(W, T + 60, 175, 0))
canvas = Image.new('RGB', (W, H))
canvas.paste(tall, (0, T))
seam = Image.new('L', (W, T + 60), 255)
seam.paste(vgrad(W, 60, 255, 0), (0, T))
canvas.paste(sky, (0, 0), seam)
canvas.save(out, quality=84, optimize=True, progressive=True)
print(canvas.size)
