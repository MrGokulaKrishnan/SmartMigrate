package com.smartmigrate.client.ui

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color

/**
 * Smart Migrate — Production Design Tokens
 * Source of truth: Logo palette on True AMOLED black (#000000).
 * Never use #050505 or any grey-black base.
 */
object SmTokens {
    // ── True AMOLED Black & Dark Surfaces ─────────────────────────────────────
    val Black = Color(0xFF000000)
    val Bg0 = Color(0xFF000000)
    val Bg1 = Color(0xFF05040B)
    val Bg2 = Color(0xFF0B0918)

    // Violet-tinted surfaces (logo-accurate)
    val Surface0 = Color(0xFF05040B)
    val Surface1 = Color(0xFF100D22)
    val Surface2 = Color(0xFF161230)
    val Surface3 = Color(0xFF1C1740)
    val Elevated1 = Color(0xFF1C1740)
    val Elevated2 = Color(0xFF221C4C)

    // Borders & dividers
    val Line = Color(0x1FFFFFFF)
    val LineSubtle = Color(0x0EFFFFFF)
    val LineMedium = Color(0x1FFFFFFF)
    val LineStrong = Color(0x554D33C4)

    // ── Brand Palette (from Smart Migrate logo) ───────────────────────────────
    val BrandPale = Color(0xFFD2CCFF)
    val BrandLight = Color(0xFFB9AEFF)
    val Brand300 = Color(0xFF7C64E6)
    val Brand400 = Color(0xFF4D33C4)
    val Brand500 = Color(0xFF4D33C4)
    val Brand600 = Color(0xFF382098)
    val Brand700 = Color(0xFF1F1266)
    val BrandGlow = Color(0x474D33C4)

    // Brand gradient: B9AEFF → 4D33C4 → 1F1266
    val BrandGradient = Brush.linearGradient(
        colors = listOf(BrandLight, Brand500, Brand700),
        start = Offset.Zero,
        end = Offset.Infinite
    )

    // ── Text Colors (WCAG AA compliant) ───────────────────────────────────────
    val TextPrimary = Color(0xFFF3F3F7)   // 18.97:1 on #000
    val TextSecondary = Color(0xFFB5B5C3) // 10.36:1 on #000
    val TextMuted = Color(0xFF757588)     // 4.66:1 on #000
    val TextAccent = Color(0xFFB9AEFF)    // 10.56:1 on #000

    // ── Status Colors (Icon + Text + Color) ───────────────────────────────────
    val Success = Color(0xFF10B981)
    val Warning = Color(0xFFF59E0B)
    val Danger = Color(0xFFEF4444)
    val Info = Color(0xFF60A5FA)
    val Offline = Color(0xFF6B7280)

    // ── Liquid Glass Material Tokens ──────────────────────────────────────────
    val GlassFillTop = Color(0x2BFFFFFF)    // 17% white
    val GlassFillBottom = Color(0x0FFFFFFF) // 6% white
    val GlassBorder = Color(0x24FFFFFF)     // 14% white border
    val GlassSpecular = Color(0x42FFFFFF)   // 26% specular highlight
}
