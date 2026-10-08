#!/usr/bin/env python3
"""
Smart Migrate — Production Multi-Platform Brand Asset Generator
Generates pixel-perfect, properly scaled, high-contrast AMOLED icons,
Windows ICO (multi-res), Android Adaptive & Legacy Mipmap icons,
and Website SVG/ICO favicons from the official Smart Migrate emblem.
"""

import os
import sys
from PIL import Image, ImageDraw, ImageEnhance

SOURCE_IMAGE = r"C:\Users\gokul\.gemini\antigravity\brain\6d9de22b-0130-49b7-80f3-9c8a3e7009af\.user_uploaded\media_1791438687860_43db8c08.jpg"
ROOT_DIR = r"C:\Smart Migrate"

def get_tight_logo_master(source_path: str) -> Image.Image:
    """
    Crops the source image tightly to the active emblem bounds with minimal safe margin,
    centering it perfectly and boosting saturation/contrast for small-screen clarity.
    """
    img = Image.open(source_path).convert("RGBA")
    gray = img.convert("L")
    thresholded = gray.point(lambda p: 255 if p > 20 else 0)
    bbox = thresholded.getbbox()
    if not bbox:
        return img

    cx = (bbox[0] + bbox[2]) / 2.0
    cy = (bbox[1] + bbox[3]) / 2.0
    side = max(bbox[2] - bbox[0], bbox[3] - bbox[1])
    
    # 6% margin so glow falloff is preserved while emblem occupies ~88% of canvas
    margin = int(side * 0.06)
    total_side = side + 2 * margin

    x1 = int(cx - total_side / 2.0)
    y1 = int(cy - total_side / 2.0)
    x2 = int(cx + total_side / 2.0)
    y2 = int(cy + total_side / 2.0)

    cropped = Image.new("RGBA", (total_side, total_side), (4, 3, 8, 255))
    src_x1 = max(0, x1)
    src_y1 = max(0, y1)
    src_x2 = min(img.width, x2)
    src_y2 = min(img.height, y2)

    sub = img.crop((src_x1, src_y1, src_x2, src_y2))
    cropped.paste(sub, (src_x1 - x1, src_y1 - y1))

    # Enhance contrast and color vibrancy for crystal-clear taskbar/launcher visibility
    enhancer_contrast = ImageEnhance.Contrast(cropped)
    cropped = enhancer_contrast.enhance(1.18)
    enhancer_color = ImageEnhance.Color(cropped)
    cropped = enhancer_color.enhance(1.15)
    
    # Resize master to a clean 1024x1024 square
    return cropped.resize((1024, 1024), Image.Resampling.LANCZOS)

def create_round_icon(image: Image.Image, size: int) -> Image.Image:
    """Creates a circular icon with antialiased edge masking."""
    scale = 4
    resized = image.resize((size * scale, size * scale), Image.Resampling.LANCZOS).convert("RGBA")
    mask = Image.new("L", (size * scale, size * scale), 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, size * scale, size * scale), fill=255)
    
    round_img = Image.new("RGBA", (size * scale, size * scale), (0, 0, 0, 0))
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

def create_adaptive_foreground(image: Image.Image, canvas_size: int) -> Image.Image:
    """
    Creates an Android Adaptive Icon foreground (108dp canvas).
    The emblem is scaled to occupy ~74% of the canvas so that within the 72dp
    safe viewport circle/squircle, the logo fills the visible space boldly.
    """
    fg = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    target_emblem_size = int(canvas_size * 0.74)
    emblem_resized = image.resize((target_emblem_size, target_emblem_size), Image.Resampling.LANCZOS)
    offset = (canvas_size - target_emblem_size) // 2
    fg.paste(emblem_resized, (offset, offset))
    return fg

def main():
    if not os.path.exists(SOURCE_IMAGE):
        print(f"Error: Source image not found at {SOURCE_IMAGE}")
        sys.exit(1)
        
    print(f"Loading source logo from: {SOURCE_IMAGE}")
    master_logo = get_tight_logo_master(SOURCE_IMAGE)
    print(f"Master tightly-scaled logo generated: {master_logo.size}")

    # ─────────────────────────────────────────────────────────────────────────
    # 1. Windows Tauri Icons (apps/windows-host/src-tauri/icons)
    # ─────────────────────────────────────────────────────────────────────────
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
        resized = master_logo.resize((w, h), Image.Resampling.LANCZOS)
        resized.save(out_path, format="PNG")
        print(f"Generated Tauri icon: {filename} ({w}x{h})")

    # Generate Windows Multi-Res icon.ico (16, 24, 32, 48, 64, 128, 256)
    ico_path = os.path.join(tauri_icons_dir, "icon.ico")
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    master_logo.save(ico_path, format="ICO", sizes=ico_sizes)
    print(f"Generated multi-res Windows icon.ico ({len(ico_sizes)} layers: 16 to 256px)")

    # Windows Host React UI logo
    win_src_dir = os.path.join(ROOT_DIR, "apps", "windows-host", "src")
    master_logo.resize((512, 512), Image.Resampling.LANCZOS).save(
        os.path.join(win_src_dir, "smart_migrate_logo.png"), format="PNG"
    )
    print("Generated Windows Host React UI logo (smart_migrate_logo.png)")

    # ─────────────────────────────────────────────────────────────────────────
    # 2. Android Client Icons (apps/android-client/app/src/main/res)
    # ─────────────────────────────────────────────────────────────────────────
    res_dir = os.path.join(ROOT_DIR, "apps", "android-client", "app", "src", "main", "res")
    os.makedirs(res_dir, exist_ok=True)

    # 2A. Adaptive icon background color in values/ic_launcher_background.xml
    values_dir = os.path.join(res_dir, "values")
    os.makedirs(values_dir, exist_ok=True)
    with open(os.path.join(values_dir, "ic_launcher_background.xml"), "w", encoding="utf-8") as f:
        f.write('''<?xml version="1.0" encoding="utf-8"?>
<resources>
    <color name="ic_launcher_background">#040308</color>
</resources>
''')

    # 2B. Adaptive icon XMLs in mipmap-anydpi-v26
    anydpi_dir = os.path.join(res_dir, "mipmap-anydpi-v26")
    os.makedirs(anydpi_dir, exist_ok=True)
    adaptive_xml = '''<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@color/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>
'''
    with open(os.path.join(anydpi_dir, "ic_launcher.xml"), "w", encoding="utf-8") as f:
        f.write(adaptive_xml)
    with open(os.path.join(anydpi_dir, "ic_launcher_round.xml"), "w", encoding="utf-8") as f:
        f.write(adaptive_xml)
    print("Generated Android Adaptive Icon XML declarations (mipmap-anydpi-v26)")

    # 2C. Mipmaps for each density: Adaptive foreground + Legacy round & squircle
    # Density definitions: (legacy_size, foreground_size)
    mipmap_configs = {
        "mipmap-mdpi": (48, 108),
        "mipmap-hdpi": (72, 162),
        "mipmap-xhdpi": (96, 216),
        "mipmap-xxhdpi": (144, 324),
        "mipmap-xxxhdpi": (192, 432),
    }

    for folder, (legacy_size, fg_size) in mipmap_configs.items():
        target_dir = os.path.join(res_dir, folder)
        os.makedirs(target_dir, exist_ok=True)

        # 1. Adaptive foreground
        fg_icon = create_adaptive_foreground(master_logo, fg_size)
        fg_icon.save(os.path.join(target_dir, "ic_launcher_foreground.png"), format="PNG")

        # 2. Legacy squircle
        square_icon = create_rounded_rect_icon(master_logo, legacy_size)
        square_icon.save(os.path.join(target_dir, "ic_launcher.png"), format="PNG")

        # 3. Legacy circle
        round_icon = create_round_icon(master_logo, legacy_size)
        round_icon.save(os.path.join(target_dir, "ic_launcher_round.png"), format="PNG")
        print(f"Generated Android {folder}: launcher ({legacy_size}px) + foreground ({fg_size}px)")

    # 2D. Android drawable logos (Splash & UI)
    drawable_dir = os.path.join(res_dir, "drawable")
    os.makedirs(drawable_dir, exist_ok=True)
    logo_512 = master_logo.resize((512, 512), Image.Resampling.LANCZOS)
    logo_512.save(os.path.join(drawable_dir, "smart_migrate_logo.png"), format="PNG")
    print("Generated Android drawable high-res logo (smart_migrate_logo.png)")

    # ─────────────────────────────────────────────────────────────────────────
    # 3. Website Assets (apps/website and apps/website/public)
    # ─────────────────────────────────────────────────────────────────────────
    web_dirs = [
        os.path.join(ROOT_DIR, "apps", "website"),
        os.path.join(ROOT_DIR, "apps", "website", "public"),
    ]
    
    for web_dir in web_dirs:
        os.makedirs(web_dir, exist_ok=True)
        
        # Favicon ICO with 16, 24, 32, 48 px
        web_ico_path = os.path.join(web_dir, "favicon.ico")
        master_logo.save(web_ico_path, format="ICO", sizes=[(16, 16), (24, 24), (32, 32), (48, 48)])
        
        # Logo PNG and JPG
        master_logo.resize((512, 512), Image.Resampling.LANCZOS).save(
            os.path.join(web_dir, "smart_migrate_logo.png"), format="PNG"
        )
        master_logo.convert("RGB").save(
            os.path.join(web_dir, "smart_migrate_logo.jpg"), format="JPEG", quality=95
        )
        
        # Apple Touch Icon (180x180 squircle)
        apple_icon = create_rounded_rect_icon(master_logo, 180, corner_radius=38)
        apple_icon.save(os.path.join(web_dir, "apple-touch-icon.png"), format="PNG")

        # Media directory if exists
        media_dir = os.path.join(web_dir, "media")
        if os.path.exists(media_dir):
            master_logo.resize((512, 512), Image.Resampling.LANCZOS).save(
                os.path.join(media_dir, "smart_migrate_logo.png"), format="PNG"
            )
            master_logo.convert("RGB").save(
                os.path.join(media_dir, "smart-migrate-logo.jpg"), format="JPEG", quality=95
            )

    # 3B. High-Contrast Scaled AMOLED Vector SVG Favicon
    # Scaled to 88% canvas fill so it is immediately legible in browser tabs
    svg_content = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <linearGradient id="purpleGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#8A6BFF"/>
      <stop offset="50%" stop-color="#5E3BEE"/>
      <stop offset="100%" stop-color="#3C1A9E"/>
    </linearGradient>
    <linearGradient id="accentGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#B39DFF"/>
      <stop offset="100%" stop-color="#6F4DFC"/>
    </linearGradient>
    <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="6" stdDeviation="14" flood-color="#6442FA" flood-opacity="0.55"/>
    </filter>
  </defs>
  <!-- Pure AMOLED Black Background -->
  <rect width="512" height="512" rx="112" fill="#040308"/>
  <rect width="510" height="510" x="1" y="1" rx="111" fill="none" stroke="#2B214E" stroke-width="2.5"/>
  
  <!-- Outer Glow Group with 1.34x scale to fill viewport crisply -->
  <g filter="url(#glow)" transform="translate(-89, -84) scale(1.34)">
    <!-- Top-Right Dual Migration Arrow Glyph -->
    <path d="M 330 145 L 360 115 L 360 135 L 405 135 L 405 155 L 360 155 L 360 175 Z" fill="url(#accentGrad)" />
    <path d="M 355 185 L 325 215 L 325 195 L 280 195 L 280 175 L 325 175 L 325 155 Z" fill="url(#accentGrad)" />
    
    <!-- Stylized "S" with Lightning Terminal -->
    <path d="M 120 180 C 120 155 140 140 185 140 L 235 140 L 235 185 L 180 185 C 168 185 162 190 162 198 C 162 208 170 212 188 216 L 215 222 C 248 230 262 248 262 278 C 262 312 238 335 195 335 L 140 335 L 115 385 L 148 335 L 120 335 C 105 335 100 322 100 310 L 142 310 C 142 322 152 328 175 328 C 195 328 215 320 215 300 C 215 285 205 278 182 272 L 155 266 C 125 258 120 238 120 212 Z" fill="url(#purpleGrad)"/>

    <!-- Stylized "M" with Lightning Terminal -->
    <path d="M 270 335 L 270 140 L 320 140 L 350 240 L 380 140 L 430 140 L 430 335 L 385 335 L 385 220 L 360 305 L 340 305 L 315 220 L 315 335 L 295 385 L 305 335 Z" fill="url(#purpleGrad)"/>
  </g>
</svg>'''

    for web_dir in web_dirs:
        svg_path = os.path.join(web_dir, "favicon.svg")
        with open(svg_path, "w", encoding="utf-8") as f:
            f.write(svg_content)
        print(f"Generated high-contrast vector favicon.svg: {svg_path}")

    print("\n[SUCCESS] All Smart Migrate multi-resolution, tightly-scaled brand assets generated across Windows, Android, and Web.")

if __name__ == "__main__":
    main()
