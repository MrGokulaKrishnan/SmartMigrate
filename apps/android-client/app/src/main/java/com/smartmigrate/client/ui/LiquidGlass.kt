package com.smartmigrate.client.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/**
 * Smart Migrate — Liquid Glass Material & Ambient Stage
 * Fills, specular rims, and stage lighting per design specification.
 */
object LiquidGlassModifiers {

    /**
     * Ambient stage lighting behind chrome on non-stream screens:
     * radial glows:
     * - rgba(77, 51, 196, 0.34) at 10% 5%
     * - rgba(185, 174, 255, 0.20) at 95% 90%
     * - rgba(124, 100, 230, 0.09) at 60% 40%
     * all over pure #000000 black.
     */
    fun Modifier.ambientStage(): Modifier = this
        .background(SmTokens.Black)
        .drawBehind {
            // Glow 1: Brand primary violet at top-left
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(Color(0x574D33C4), Color.Transparent),
                    center = Offset(size.width * 0.10f, size.height * 0.05f),
                    radius = size.width * 1.1f
                )
            )
            // Glow 2: Light accent at bottom-right
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(Color(0x33B9AEFF), Color.Transparent),
                    center = Offset(size.width * 0.95f, size.height * 0.90f),
                    radius = size.width * 0.9f
                )
            )
            // Glow 3: Soft mid violet at center-right
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(Color(0x177C64E6), Color.Transparent),
                    center = Offset(size.width * 0.60f, size.height * 0.40f),
                    radius = size.width * 0.7f
                )
            )
        }

    /**
     * Default Liquid Glass Surface:
     * - Fill: linear-gradient(180deg, rgba(185,174,255,0.17), rgba(77,51,196,0.06))
     * - Border: 1 px rgba(255,255,255,0.14)
     * - Top specular: linear-gradient(180deg, rgba(255,255,255,0.26), transparent 45%) at 60% opacity
     * - Inner highlight: inset 0 1px 0 rgba(255,255,255,0.32) and inset 0 -1px 0 rgba(0,0,0,0.35)
     * - Outer shadow: 0 14px 40px rgba(0,0,0,0.5)
     */
    fun Modifier.liquidGlass(
        shape: Shape = RoundedCornerShape(24.dp),
        isLightweight: Boolean = false,
        elevation: Dp = 12.dp
    ): Modifier {
        return if (isLightweight) {
            this
                .shadow(elevation = elevation, shape = shape)
                .clip(shape)
                .background(Color(0xF8181820))
                .border(1.dp, Color(0x33FFFFFF), shape)
        } else {
            this
                .shadow(
                    elevation = elevation,
                    shape = shape,
                    ambientColor = Color.Black,
                    spotColor = Color(0x664D33C4)
                )
                .clip(shape)
                .background(
                    Brush.verticalGradient(
                        colors = listOf(
                            Color(0x2BFFFFFF), // 17% white top
                            Color(0x0FFFFFFF)  // 6% white bottom
                        )
                    )
                )
                .border(
                    width = 1.dp,
                    color = Color(0x24FFFFFF), // 14% white border
                    shape = shape
                )
                .drawBehind {
                    // Top specular highlight
                    drawLine(
                        color = Color(0x52FFFFFF), // 32% specular
                        start = Offset(16.dp.toPx(), 1.dp.toPx()),
                        end = Offset(size.width - 16.dp.toPx(), 1.dp.toPx()),
                        strokeWidth = 1.dp.toPx()
                    )
                }
        }
    }

    /**
     * Hero card with extra glow layer under the glass:
     * linear-gradient(160deg, rgba(77,51,196,0.36), rgba(0,0,0,0) 62%)
     */
    fun Modifier.heroGlass(
        shape: Shape = RoundedCornerShape(28.dp),
        isLightweight: Boolean = false
    ): Modifier {
        return this
            .liquidGlass(shape = shape, isLightweight = isLightweight, elevation = 16.dp)
            .background(
                Brush.linearGradient(
                    colors = listOf(
                        Color(0x5C4D33C4), // 36% violet
                        Color.Transparent
                    ),
                    start = Offset.Zero,
                    end = Offset(400f, 600f)
                )
            )
    }
}
