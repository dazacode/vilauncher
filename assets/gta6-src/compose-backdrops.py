"""
Builds the library backgrounds in public/gta6/art from Rockstar's official
"Artwork & Wallpapers" pack (rockstargames.com/VI/media, GTAVI_Artwork_Wallpapers.zip).

  python3 compose-backdrops.py <folder with the *_landscape.jpg files> ../../public/gta6/art

Each picture is scaled to 1440 px high, given 14% more sky above it and a soft, blurred extension on the left (a quarter of its width),
so the page can slide the subject right of the title column and below the nav. src/lib/backdrops.ts holds each picture's aspect and where its
subject stands; keep them in step if this changes.
"""
import os, sys
from PIL import Image, ImageFilter, ImageOps

PICKS = {
    "boobie-ike": "Boobie_Ike",
    "brian-heder": "Brian_Heder",
    "cal-hampton": "Cal_Hampton",
    "drequan-priest": "DreQuan_Priest",
    "rooftop": "Jason_Lucia_Rooftop",
    "jason-lucia-01": "Jason_and_Lucia_01",
    "beach": "Jason_and_Lucia_Beach",
    "mount-kalaga": "Mount_Kalaga_National_Park_Postcard",
}
H = 1440
EXT = 0.25
# Room above the subject, as on the Robbery art: the top rows carried up, smoothed and darkened.
TOP = 0.14


def postcard(im):
    """The photo inside the tilted postcard, straightened."""
    im = im.rotate(-3.35, resample=Image.BICUBIC, center=(1921, 1085))
    return im.crop((721, 305, 3121, 1865))


def hgrad(w, h, a, b):
    g = Image.linear_gradient("L").rotate(90, expand=True).resize((w, h))
    return g.point(lambda v: int(a + (b - a) * v / 255))


def vgrad(w, h, a, b):
    g = Image.linear_gradient("L").resize((w, h))
    return g.point(lambda v: int(a + (b - a) * v / 255))


def extend_top(im):
    w, h = im.size
    T = int(h * TOP)
    row = im.crop((0, 0, w, 12)).resize((w, 1), Image.BOX)
    row = row.resize((w, 40)).filter(ImageFilter.GaussianBlur(60)).resize((w, 1), Image.BOX)
    sky = row.resize((w, T + 60), Image.BILINEAR)
    dark = Image.new("RGB", sky.size, (22, 16, 36))
    sky = Image.composite(dark, sky, vgrad(w, T + 60, 170, 0))
    out = Image.new("RGB", (w, h + T))
    out.paste(im, (0, T))
    seam = Image.new("L", sky.size, 255)
    seam.paste(vgrad(w, 60, 255, 0), (0, T))
    out.paste(sky, (0, 0), seam)
    return out


def extend_left(im):
    w, h = im.size
    L = int(w * EXT)
    wide = Image.new("RGB", (L + w, h))
    wide.paste(ImageOps.mirror(im.crop((0, 0, L, h))), (0, 0))
    wide.paste(im, (L, 0))
    soft = wide.filter(ImageFilter.GaussianBlur(40))
    m = Image.new("L", wide.size, 0)
    m.paste(255, (0, 0, L, h))
    m.paste(hgrad(int(w * 0.1), h, 255, 0), (L, 0))
    return Image.composite(soft, wide, m)


src, out = sys.argv[1:3]
os.makedirs(os.path.join(out, "thumbs"), exist_ok=True)
for slug, name in PICKS.items():
    im = Image.open(os.path.join(src, name + "_landscape.jpg")).convert("RGB")
    if slug == "mount-kalaga":
        im = postcard(im)
    im = im.resize((round(im.width * H / im.height), H), Image.LANCZOS)
    thumb = im.resize((320, round(320 * im.height / im.width)), Image.LANCZOS)
    im = extend_top(im)
    im = extend_left(im)
    im.save(os.path.join(out, slug + ".jpg"), quality=80, optimize=True, progressive=True)
    thumb.save(os.path.join(out, "thumbs", slug + ".jpg"), quality=80, optimize=True)
    print(slug, im.size, round(im.width / im.height, 4))
