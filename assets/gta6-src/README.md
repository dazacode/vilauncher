# GTA VI art sources

Third-party art: the Grand Theft Auto VI artwork (the "Robbery" key art in `robbery.jpg` and the pieces from the official wallpaper pack) and logo belong to Rockstar Games / Take-Two Interactive. VILauncher shows them only to identify the game on its library page.

What ships in `public/gta6/` is made from these:

- `art/*.jpg` and `art/thumbs/*.jpg`: the other library backgrounds, from Rockstar's official "Artwork & Wallpapers" pack (rockstargames.com/VI/media, `GTAVI_Artwork_Wallpapers.zip`, the `*_landscape.jpg` files). Unzip the pack, then `python3 compose-backdrops.py <folder with the landscape files> ../../public/gta6/art`. Their aspect and where each subject stands are listed in `src/lib/backdrops.ts`; the picker is in Settings → Appearance.

- `hero-bg.jpg`: `python3 compose-hero.py robbery.jpg ../../public/gta6/hero-bg.jpg` (Pillow). The art with its sky carried up 452 px and its left edge extended 147 px as a soft haze, so the page can show it full height with Jason and Lucia where the design has them. `.gta-art` in `src/styles.css` depends on its size (2707 × 2000): keep the aspect (1.3535) and Jason's position (0.7067 across) in step if you regenerate it.
- `gta-tile.jpg`: `python3 compose-tile.py robbery.jpg ../../public/gta6/gta-tile.jpg`, a 3 × 3 collage (used on the Workshop page's example cards).
- `logo.svg`: the vector logo from [Dashboard Icons](https://github.com/homarr-labs/dashboard-icons) (`svg/grand-theft-auto-vi.svg`), with a layer of palm silhouettes clipped to the V and I added after the gradient fills.
