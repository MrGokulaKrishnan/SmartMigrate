package com.smartmigrate.client.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

enum class GlossyPalette(val top: Color, val bottom: Color) {
    BrandViolet(Color(0xFF8D7BFF), Color(0xFF382098)),
    Lavender(Color(0xFFB9AEFF), Color(0xFF4D33C4)),
    Indigo(Color(0xFF7C64E6), Color(0xFF1F1266)),
    Blue(Color(0xFF8FA8FF), Color(0xFF3A4FD0)),
    Neutral(Color(0xFF9CA3AF), Color(0xFF374151)),
    Warning(Color(0xFFFCD34D), Color(0xFFD97706)),
    Success(Color(0xFF86EFAC), Color(0xFF15803D)),
    Destructive(Color(0xFFFCA5A5), Color(0xFFDC2626)),
}

enum class GlyphType {
    Monitor,
    Laptop,
    Bolt,
    Shield,
    Lock,
    Info,
    Check,
    Plus,
    Sliders,
    Phone,
    Refresh,
    Close,
}

@Composable
fun GlossyIconTile(
    glyph: GlyphType,
    modifier: Modifier = Modifier,
    palette: GlossyPalette = GlossyPalette.BrandViolet,
    size: Dp = 48.dp
) {
    Canvas(modifier = modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val scale = w / 64f

        val cornerR = 18f * scale

        // 1. Tile Base with Vertical Linear Gradient
        val tilePath = Path().apply {
            addRoundRect(
                RoundRect(
                    rect = Rect(0f, 0f, w, h),
                    cornerRadius = CornerRadius(cornerR, cornerR)
                )
            )
        }

        drawPath(
            path = tilePath,
            brush = Brush.verticalGradient(
                colors = listOf(palette.top, palette.bottom),
                startY = 0f,
                endY = h
            )
        )

        // 2. Bevel border (inset 0.75, stroke 1.5)
        val bevelInset = 0.75f * scale
        val bevelR = 17.5f * scale
        val bevelPath = Path().apply {
            addRoundRect(
                RoundRect(
                    rect = Rect(bevelInset, bevelInset, w - bevelInset, h - bevelInset),
                    cornerRadius = CornerRadius(bevelR, bevelR)
                )
            )
        }
        drawPath(
            path = bevelPath,
            color = Color.White.copy(alpha = 0.24f),
            style = Stroke(width = 1.5f * scale)
        )

        // 3. Top Specular Gloss
        // M1.5 18 Q1.5 2 18 2 H46 Q62.5 2 62.5 18 V31 Q32 42 1.5 31 Z
        clipPath(tilePath) {
            val glossPath = Path().apply {
                moveTo(1.5f * scale, 18f * scale)
                quadraticTo(1.5f * scale, 2f * scale, 18f * scale, 2f * scale)
                lineTo(46f * scale, 2f * scale)
                quadraticTo(62.5f * scale, 2f * scale, 62.5f * scale, 18f * scale)
                lineTo(62.5f * scale, 31f * scale)
                quadraticTo(32f * scale, 42f * scale, 1.5f * scale, 31f * scale)
                close()
            }
            drawPath(
                path = glossPath,
                brush = Brush.verticalGradient(
                    colors = listOf(Color.White.copy(alpha = 0.58f), Color.White.copy(alpha = 0.0f)),
                    startY = 0f,
                    endY = 36f * scale
                )
            )
        }

        // 4. White Glyph (24-unit icon scaled by 1.333 and translated to 16,16 in 64px coords)
        val strokeWidth = 1.7f * scale
        val stroke = Stroke(
            width = strokeWidth,
            cap = StrokeCap.Round,
            join = StrokeJoin.Round
        )

        val gx = 16f * scale
        val gy = 16f * scale
        val gs = 1.333f * scale

        fun p(x: Float, y: Float) = Offset(gx + x * gs, gy + y * gs)

        when (glyph) {
            GlyphType.Monitor -> {
                drawRoundRect(
                    color = Color.White,
                    topLeft = p(2f, 3f),
                    size = Size(20f * gs, 13f * gs),
                    cornerRadius = CornerRadius(2f * gs, 2f * gs),
                    style = stroke
                )
                drawLine(Color.White, p(8f, 21f), p(16f, 21f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawLine(Color.White, p(12f, 16f), p(12f, 21f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
            }
            GlyphType.Laptop -> {
                drawRoundRect(
                    color = Color.White,
                    topLeft = p(4f, 5f),
                    size = Size(16f * gs, 11f * gs),
                    cornerRadius = CornerRadius(2f * gs, 2f * gs),
                    style = stroke
                )
                drawLine(Color.White, p(2f, 19f), p(22f, 19f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
            }
            GlyphType.Bolt -> {
                val bolt = Path().apply {
                    moveTo(gx + 13f * gs, gy + 2f * gs)
                    lineTo(gx + 4f * gs, gy + 14f * gs)
                    lineTo(gx + 11f * gs, gy + 14f * gs)
                    lineTo(gx + 10f * gs, gy + 22f * gs)
                    lineTo(gx + 19f * gs, gy + 10f * gs)
                    lineTo(gx + 12f * gs, gy + 10f * gs)
                    close()
                }
                drawPath(bolt, Color.White, style = stroke)
            }
            GlyphType.Shield -> {
                val shield = Path().apply {
                    moveTo(gx + 12f * gs, gy + 3f * gs)
                    lineTo(gx + 20f * gs, gy + 6f * gs)
                    lineTo(gx + 20f * gs, gy + 12f * gs)
                    cubicTo(gx + 20f * gs, gy + 17f * gs, gx + 16.5f * gs, gy + 20f * gs, gx + 12f * gs, gy + 21f * gs)
                    cubicTo(gx + 7.5f * gs, gy + 20f * gs, gx + 4f * gs, gy + 17f * gs, gx + 4f * gs, gy + 12f * gs)
                    lineTo(gx + 4f * gs, gy + 6f * gs)
                    close()
                }
                drawPath(shield, Color.White, style = stroke)
            }
            GlyphType.Lock -> {
                drawRoundRect(
                    color = Color.White,
                    topLeft = p(5f, 11f),
                    size = Size(14f * gs, 10f * gs),
                    cornerRadius = CornerRadius(2f * gs, 2f * gs),
                    style = stroke
                )
                val shackle = Path().apply {
                    moveTo(gx + 8f * gs, gy + 11f * gs)
                    lineTo(gx + 8f * gs, gy + 7f * gs)
                    cubicTo(gx + 8f * gs, gy + 4.8f * gs, gx + 9.8f * gs, gy + 3f * gs, gx + 12f * gs, gy + 3f * gs)
                    cubicTo(gx + 14.2f * gs, gy + 3f * gs, gx + 16f * gs, gy + 4.8f * gs, gx + 16f * gs, gy + 7f * gs)
                    lineTo(gx + 16f * gs, gy + 11f * gs)
                }
                drawPath(shackle, Color.White, style = stroke)
            }
            GlyphType.Info -> {
                drawCircle(Color.White, radius = 9f * gs, center = p(12f, 12f), style = stroke)
                drawLine(Color.White, p(12f, 11f), p(12f, 16f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawCircle(Color.White, radius = 1f * scale, center = p(12f, 8f))
            }
            GlyphType.Check -> {
                val check = Path().apply {
                    moveTo(gx + 5f * gs, gy + 12.5f * gs)
                    lineTo(gx + 9.5f * gs, gy + 17f * gs)
                    lineTo(gx + 19f * gs, gy + 7.5f * gs)
                }
                drawPath(check, Color.White, style = stroke)
            }
            GlyphType.Plus -> {
                drawLine(Color.White, p(12f, 5f), p(12f, 19f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawLine(Color.White, p(5f, 12f), p(19f, 12f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
            }
            GlyphType.Sliders -> {
                drawLine(Color.White, p(4f, 6f), p(14f, 6f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawLine(Color.White, p(18f, 6f), p(20f, 6f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawCircle(Color.White, radius = 2f * gs, center = p(16f, 6f), style = stroke)

                drawLine(Color.White, p(4f, 12f), p(8f, 12f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawLine(Color.White, p(12f, 12f), p(20f, 12f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawCircle(Color.White, radius = 2f * gs, center = p(10f, 12f), style = stroke)

                drawLine(Color.White, p(4f, 18f), p(16f, 18f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawCircle(Color.White, radius = 2f * gs, center = p(18f, 18f), style = stroke)
            }
            GlyphType.Phone -> {
                drawRoundRect(
                    color = Color.White,
                    topLeft = p(6f, 2f),
                    size = Size(12f * gs, 20f * gs),
                    cornerRadius = CornerRadius(3f * gs, 3f * gs),
                    style = stroke
                )
                drawLine(Color.White, p(11f, 18f), p(13f, 18f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
            }
            GlyphType.Refresh -> {
                val arc = Path().apply {
                    moveTo(gx + 20f * gs, gy + 11f * gs)
                    cubicTo(gx + 19.5f * gs, gy + 6.5f * gs, gx + 15.5f * gs, gy + 3f * gs, gx + 11f * gs, gy + 3.5f * gs)
                    cubicTo(gx + 6.5f * gs, gy + 4f * gs, gx + 3.5f * gs, gy + 8f * gs, gx + 4f * gs, gy + 12.5f * gs)
                }
                drawPath(arc, Color.White, style = stroke)
            }
            GlyphType.Close -> {
                drawLine(Color.White, p(6f, 6f), p(18f, 18f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
                drawLine(Color.White, p(18f, 6f), p(6f, 18f), strokeWidth = strokeWidth, cap = StrokeCap.Round)
            }
        }
    }
}
