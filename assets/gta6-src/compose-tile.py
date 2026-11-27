
import sys
from PIL import Image, ImageDraw, ImageEnhance
src, out = sys.argv[1:3]
R = Image.open(src).convert('RGB')
cells = [(1180,160,330),(1660,30,330),(1780,520,380),(250,520,300),None,(1700,380,330),(30,1060,330),(880,780,380),(1560,1000,420)]
S, G = 128, 3
c = (S - 2*G) // 3
T = Image.new('RGB', (S, S), (22, 12, 34))
for i, box in enumerate(cells):
    x, y = (i % 3) * (c + G), (i // 3) * (c + G)
    if box is None:
        g = Image.new('RGB', (c, c))
        d = ImageDraw.Draw(g)
        for yy in range(c):
            t = yy / (c - 1)
            col = tuple(int(a + (b - a) * t) for a, b in zip((52, 92, 214), (238, 150, 70)))
            if t < 0.5:
                col = tuple(int(a + (b - a) * t * 2) for a, b in zip((52, 92, 214), (255, 76, 185)))
            else:
                col = tuple(int(a + (b - a) * (t - 0.5) * 2) for a, b in zip((255, 76, 185), (230, 149, 62)))
            d.line([(0, yy), (c, yy)], fill=col)
        d.rectangle([c*0.36, c*0.3, c*0.64, c*0.7], fill=(255, 255, 255))
        T.paste(g, (x, y))
        continue
    bx, by, bs = box
    T.paste(R.crop((bx, by, bx + bs, by + bs)).resize((c, c), Image.LANCZOS), (x, y))
T = ImageEnhance.Color(T).enhance(1.15)
T.save(out, quality=88)
