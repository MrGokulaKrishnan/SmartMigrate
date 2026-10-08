#!/usr/bin/env python3
"""
Generate all multi-resolution brand assets, Windows ICO, Android mipmap icons,
and website favicons from the official Smart Migrate brand reference image.
"""

import os
import sys
from PIL import Image, ImageDraw

SOURCE_IMAGE = r"C:\Users\gokul\.gemini\antigravity\brain\6d9de22b-0130-49b7-80f3-9c8a3e7009af\.user_uploaded\media_1791438687860_43db8c08.jpg"
ROOT_DIR = r"C:\Smart Migrate"

def create_round_icon(image: Image.Image, size: int) -> Image.Image:
    """Creates a circular icon with antialiased edge masking."""
    resized = image.resize((size * 4, size * 4), Image.Resampling.LANCZOS).convert("RGBA")
    mask = Image.new("L", (size * 4, size * 4), 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, size * 4, size * 4), fill=255)
    
    round_img = Image.new("RGBA", (size * 4, size * 4), (0, 0, 0, 0))
    round_img.paste(resized, (0, 0), mask=mask)
    return round_img.resize((size, size), Image.Resampling.LANCZOS)

def create_rounded_rect_icon(image: Image.Image, size: int, corner_radius: int = None) -> Image.Image:
    """Creates a squirclish / rounded rectangle icon."""
    if corner_radius is None:
        corner_radius = max(4, int(size * 0.22))
    
    scale = 4
    resized = image.resize((size * scale, size * scale), Image.Resampling.LANCZOS).convert("RGBA")
    mask = Image.new("L", (size * scale, size * scale), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, size * scale, size * scale), radius=corner_radius * scale, fill=255)
    
    rect_img = Image.new("RGBA", (size * scale, size * scale), (0, 0, 0, 0))
    rect_img.paste(resized, (0, 0), mask=mask)
    return rect_img.resize((size, size), Image.Resampling.LANCZOS)

def main():
    if not os.path.exists(SOURCE_IMAGE):
        print(f"Error: Source image not found at {SOURCE_IMAGE}")
        sys.exit(1)
        
    print(f"Loading source logo from: {SOURCE_IMAGE}")
    base_img = Image.open(SOURCE_IMAGE).convert("RGBA")
    print(f"Source image loaded: {base_img.size} {base_img.mode}")

    # 1. Windows Tauri Icons (apps/windows-host/src-tauri/icons)
    tauri_icons_dir = os.path.join(ROOT_DIR, "apps", "windows-host", "src-tauri", "icons")
    os.makedirs(tauri_icons_dir, exist_ok=True)
    
    tauri_sizes = {
        "icon.png": (512, 512),
        "32x32.png": (32, 32),
        "64x64.png": (64, 64),
        "128x128.png": (128, 128),
        "128x128@2x.png": (256, 256),
        "Square30x30Logo.png": (30, 30),
        "Square44x44Logo.png": (44, 44),
        "Square71x71Logo.png": (71, 71),
        "Square89x89Logo.png": (89, 89),
        "Square107x107Logo.png": (107, 107),
        "Square142x142Logo.png": (142, 142),
        "Square150x150Logo.png": (150, 150),
        "Square284x284Logo.png": (284, 284),
        "Square310x310Logo.png": (310, 310),
        "StoreLogo.png": (50, 50),
    }
    
    for filename, (w, h) in tauri_sizes.items():
        out_path = os.path.join(tauri_icons_dir, filename)
        resized = base_img.resize((w, h), Image.Resampling.LANCZOS)
        resized.save(out_path, format="PNG")
        print(f"Generated Tauri icon: {filename} ({w}x{h})")

    # Generate multi-res icon.ico
    ico_path = os.path.join(tauri_icons_dir, "icon.ico")
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    base_img.save(ico_path, format="ICO", sizes=ico_sizes)
    print(f"Generated multi-res icon.ico ({len(ico_sizes)} layers)")

    # 2. Android Client Icons (apps/android-client/app/src/main/res)
    res_dir = os.path.join(ROOT_DIR, "apps", "android-client", "app", "src", "main", "res")
    mipmap_densities = {
        "mipmap-mdpi": 48,
        "mipmap-hdpi": 72,
        "mipmap-xhdpi": 96,
        "mipmap-xxhdpi": 144,
        "mipmap-xxxhdpi": 192,
    }
    
    for folder, size in mipmap_densities.items():
        target_dir = os.path.join(res_dir, folder)
        os.makedirs(target_dir, exist_ok=True)
        
        # Square / squircle icon
        square_icon = create_rounded_rect_icon(base_img, size)
        square_icon.save(os.path.join(target_dir, "ic_launcher.png"), format="PNG")
        
        # Round icon
        round_icon = create_round_icon(base_img, size)
        round_icon.save(os.path.join(target_dir, "ic_launcher_round.png"), format="PNG")
        print(f"Generated Android {folder} ({size}x{size})")

    # Android drawable logos
    drawable_dir = os.path.join(res_dir, "drawable")
    os.makedirs(drawable_dir, exist_ok=True)
    logo_512 = base_img.resize((512, 512), Image.Resampling.LANCZOS)
    logo_512.save(os.path.join(drawable_dir, "smart_migrate_logo.png"), format="PNG")
    logo_512.convert("RGB").save(os.path.join(drawable_dir, "smart_migrate_logo.jpg"), format="JPEG", quality=95)
    print("Generated Android drawable logos (smart_migrate_logo.png/.jpg)")

    # 3. Windows React UI Assets
    win_src_dir = os.path.join(ROOT_DIR, "apps", "windows-host", "src")
    base_img.resize((512, 512), Image.Resampling.LANCZOS).save(
        os.path.join(win_src_dir, "smart_migrate_logo.png"), format="PNG"
    )
    print("Generated Windows Host React logo (smart_migrate_logo.png)")

    # 4. Website Assets (apps/website/public)
    website_public_dir = os.path.join(ROOT_DIR, "apps", "website", "public")
    os.makedirs(website_public_dir, exist_ok=True)
    
    # Save website favicon.ico
    web_ico_path = os.path.join(website_public_dir, "favicon.ico")
    base_img.save(web_ico_path, format="ICO", sizes=[(16, 16), (32, 32), (48, 48)])
    
    # Save website logo
    base_img.resize((512, 512), Image.Resampling.LANCZOS).save(
        os.path.join(website_public_dir, "smart_migrate_logo.png"), format="PNG"
    )
    base_img.convert("RGB").save(
        os.path.join(website_public_dir, "smart_migrate_logo.jpg"), format="JPEG", quality=95
    )
    
    # Also save in website media dir if present
    web_media_dir = os.path.join(website_public_dir, "media")
    if os.path.exists(web_media_dir):
        base_img.resize((512, 512), Image.Resampling.LANCZOS).save(
            os.path.join(web_media_dir, "smart_migrate_logo.png"), format="PNG"
        )
    print("Generated Website public branding assets")

    # 5. Crisp AMOLED SVG Favicon
    svg_content = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="purpleGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#7B5EFF"/>
      <stop offset="50%" stop-color="#5538EE"/>
      <stop offset="100%" stop-color="#3A1E9E"/>
    </linearGradient>
    <linearGradient id="accentGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#9E86FF"/>
      <stop offset="100%" stop-color="#6442FA"/>
    </linearGradient>
    <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="8" stdDeviation="12" flood-color="#6442FA" flood-opacity="0.45"/>
    </filter>
  </defs>
  <!-- Pure AMOLED Black Background -->
  <rect width="512" height="512" rx="112" fill="#040308"/>
  <rect width="510" height="510" x="1" y="1" rx="111" fill="none" stroke="#251D42" stroke-width="2"/>
  
  <!-- Outer Glow Group -->
  <g filter="url(#glow)">
    <!-- Top-Right Dual Migration Arrow Glyph -->
    <path d="M 330 145 L 360 115 L 360 135 L 405 135 L 405 155 L 360 155 L 360 175 Z" fill="url(#accentGrad)" />
    <path d="M 355 185 L 325 215 L 325 195 L 280 195 L 280 175 L 325 175 L 325 155 Z" fill="url(#accentGrad)" />
    
    <!-- Stylized "S" with Lightning Terminal -->
    <path d="M 120 180 C 120 155 140 140 185 140 L 235 140 L 235 185 L 180 185 C 168 185 162 190 162 198 C 162 208 170 212 188 216 L 215 222 C 248 230 262 248 262 278 C 262 312 238 335 195 335 L 140 335 L 115 385 L 148 335 L 120 335 C 105 335 100 322 100 310 L 142 310 C 142 322 152 328 175 328 C 195 328 215 320 215 300 C 215 285 205 278 182 272 L 155 266 C 125 258 120 238 120 212 Z" fill="url(#purpleGrad)"/>

    <!-- Stylized "M" with Lightning Terminal -->
    <path d="M 270 335 L 270 140 L 320 140 L 350 240 L 380 140 L 430 140 L 430 335 L 385 335 L 385 220 L 360 305 L 340 305 L 315 220 L 315 335 L 295 385 L 305 335 Z" fill="url(#purpleGrad)"/>
  </g>
</svg>'''
    
    svg_path = os.path.join(website_public_dir, "favicon.svg")
    with open(svg_path, "w", encoding="utf-8") as f:
        f.write(svg_content)
    print(f"Generated vector favicon.svg: {svg_path}")

    print("\n[SUCCESS] All Smart Migrate brand assets generated successfully across all platforms.")

if __name__ == "__main__":
    main()
