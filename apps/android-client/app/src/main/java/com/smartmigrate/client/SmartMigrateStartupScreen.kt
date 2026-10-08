package com.smartmigrate.client

import android.provider.Settings
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.coerceIn
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlin.math.cos
import kotlin.math.sin

// Smart Migrate Official Purple / Violet Palette
private val AmoledBlack = Color(0xFF000000)
private val DeepPurple = Color(0xFF30127A)
private val MidnightViolet = Color(0xFF4526B8)
private val ElectricViolet = Color(0xFF5B35D5)
private val VividPurple = Color(0xFF725CFF)
private val HighlightLavender = Color(0xFF927CFF)
private val GlowWhite = Color(0xFFF2EFFF)
private val GlassBorder = Color(0x55BDB4FF)

/**
 * Premium cinematic Android startup experience for Smart Migrate.
 * Pure AMOLED black foundation with a smooth purple/violet ambient reveal,
 * emerging Smart Migrate emblem, illuminated migration paths, clean typography,
 * subtle violet light sweep, and smooth transition into the application.
 */
@Composable
fun SmartMigrateStartupScreen(
    onStartupFinished: () -> Unit
) {
    val context = LocalContext.current
    val isReducedMotion = remember {
        try {
            val scale = Settings.Global.getFloat(
                context.contentResolver,
                Settings.Global.ANIMATOR_DURATION_SCALE,
                1.0f
            )
            scale == 0f
        } catch (_: Exception) {
            false
        }
    }

    // Animation progress from 0f to 1f over 1800ms (or 600ms for reduced motion)
    val animProgress = remember { Animatable(0f) }
    val exitAlpha = remember { Animatable(1f) }

    LaunchedEffect(Unit) {
        if (isReducedMotion) {
            animProgress.animateTo(
                targetValue = 1f,
                animationSpec = tween(durationMillis = 600, easing = LinearEasing)
            )
            exitAlpha.animateTo(
                targetValue = 0f,
                animationSpec = tween(durationMillis = 200, easing = LinearEasing)
            )
            onStartupFinished()
        } else {
            animProgress.animateTo(
                targetValue = 1f,
                animationSpec = tween(durationMillis = 1800, easing = FastOutSlowInEasing)
            )
            exitAlpha.animateTo(
                targetValue = 0f,
                animationSpec = tween(durationMillis = 300, easing = FastOutSlowInEasing)
            )
            onStartupFinished()
        }
    }

    val progress = animProgress.value

    // Stage 1: Ambient purple center glow (0.00s - 0.35s)
    val ambientGlowAlpha = (progress / 0.30f).coerceIn(0f, 1f) * 0.65f
    val ambientGlowRadiusScale = 0.6f + (progress * 0.4f)

    // Stage 2: Logo emergence (0.35s - 0.65s)
    val logoEmergence = ((progress - 0.35f) / 0.30f).coerceIn(0f, 1f)
    val logoAlpha = logoEmergence
    val logoScale = 0.80f + (logoEmergence * 0.20f)
    val logoOffsetY = ((1f - logoEmergence) * 14f).dp

    // Stage 3: Migration arrows / connection stream (0.60s - 0.90s)
    val arrowsProgress = ((progress - 0.60f) / 0.30f).coerceIn(0f, 1f)
    val arrowsAlpha = arrowsProgress

    // Stage 4: Typography reveal (0.90s - 1.20s)
    val textProgress = ((progress - 0.90f) / 0.30f).coerceIn(0f, 1f)
    val textAlpha = textProgress
    val textOffsetY = ((1f - textProgress) * 10f).dp

    // Stage 5: Violet light sweep (1.20s - 1.50s)
    val sweepProgress = ((progress - 1.20f) / 0.30f).coerceIn(0f, 1f)
    val sweepAlpha = if (progress in 1.20f..1.55f && !isReducedMotion) {
        val p = (progress - 1.20f) / 0.35f
        if (p < 0.5f) p * 2f else (1f - p) * 2f
    } else 0f

    BoxWithConstraints(
        modifier = Modifier
            .fillMaxSize()
            .background(AmoledBlack)
            .alpha(exitAlpha.value)
    ) {
        val screenWidth = maxWidth

        // 1. Cinematic Background Glow Layer (AMOLED Black + Soft Radial Purple Glows)
        Canvas(modifier = Modifier.fillMaxSize()) {
            val centerOffset = Offset(size.width * 0.5f, size.height * 0.44f)
            val maxRadius = size.minDimension * 0.85f * ambientGlowRadiusScale

            // Primary deep velvet purple core glow
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(
                        ElectricViolet.copy(alpha = 0.38f * ambientGlowAlpha),
                        MidnightViolet.copy(alpha = 0.24f * ambientGlowAlpha),
                        DeepPurple.copy(alpha = 0.12f * ambientGlowAlpha),
                        Color.Transparent
                    ),
                    center = centerOffset,
                    radius = maxRadius
                ),
                center = centerOffset,
                radius = maxRadius
            )

            // Dynamic secondary highlight accent at top right
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(
                        VividPurple.copy(alpha = 0.18f * ambientGlowAlpha),
                        Color.Transparent
                    ),
                    center = Offset(size.width * 0.82f, size.height * 0.25f),
                    radius = maxRadius * 0.7f
                ),
                center = Offset(size.width * 0.82f, size.height * 0.25f),
                radius = maxRadius * 0.7f
            )
        }

        // 2. Subtle Micro-Particles (Gentle ambient technical dust, suppressed in reduced motion)
        if (!isReducedMotion && progress > 0.12f) {
            Canvas(modifier = Modifier.fillMaxSize()) {
                val particleAlpha = ((progress - 0.12f) / 0.25f).coerceIn(0f, 1f) * 0.28f
                val count = 10
                for (i in 0 until count) {
                    val angle = (i * (360f / count) + (progress * 40f)) * (Math.PI / 180f).toFloat()
                    val dist = (size.minDimension * 0.28f) + (i * 12f)
                    val px = (size.width * 0.5f) + cos(angle) * dist
                    val py = (size.height * 0.44f) + sin(angle) * (dist * 0.7f)
                    drawCircle(
                        color = HighlightLavender.copy(alpha = particleAlpha * (0.4f + (i % 3) * 0.2f)),
                        radius = (1.5f + (i % 2) * 1.0f).dp.toPx(),
                        center = Offset(px, py)
                    )
                }
            }
        }

        // 3. Central Brand Stage: Emblem + Migration Paths + Typography
        Column(
            modifier = Modifier
                .align(Alignment.Center)
                .offset(y = (-20).dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            // Responsive emblem size: 28% of screen width, clamped between 96.dp and 130.dp
            val emblemSize = (screenWidth * 0.30f).coerceIn(96.dp, 130.dp)

            Box(
                modifier = Modifier
                    .size(emblemSize)
                    .offset(y = logoOffsetY)
                    .scale(logoScale)
                    .alpha(logoAlpha),
                contentAlignment = Alignment.Center
            ) {
                // Outer soft halo
                Box(
                    modifier = Modifier
                        .size(emblemSize + 24.dp)
                        .blur(20.dp)
                        .background(
                            brush = Brush.radialGradient(
                                colors = listOf(
                                    ElectricViolet.copy(alpha = 0.50f),
                                    VividPurple.copy(alpha = 0.25f),
                                    Color.Transparent
                                )
                            ),
                            shape = CircleShape
                        )
                )

                // Glass squircle container
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .clip(RoundedCornerShape(26.dp, 8.dp, 26.dp, 8.dp))
                        .background(
                            brush = Brush.linearGradient(
                                colors = listOf(
                                    Color(0xFF140F2D).copy(alpha = 0.85f),
                                    Color(0xFF090616).copy(alpha = 0.95f)
                                )
                            )
                        )
                        .border(
                            width = 1.5.dp,
                            brush = Brush.linearGradient(
                                colors = listOf(
                                    HighlightLavender.copy(alpha = 0.85f),
                                    ElectricViolet.copy(alpha = 0.45f),
                                    GlassBorder
                                )
                            ),
                            shape = RoundedCornerShape(26.dp, 8.dp, 26.dp, 8.dp)
                        ),
                    contentAlignment = Alignment.Center
                ) {
                    // Smart Migrate official brand logo image
                    Image(
                        painter = painterResource(R.drawable.smart_migrate_logo),
                        contentDescription = "Smart Migrate Logo",
                        modifier = Modifier
                            .size(emblemSize * 0.72f)
                            .clip(RoundedCornerShape(14.dp, 5.dp, 14.dp, 5.dp))
                    )

                    // Light sweep effect (passes across the emblem from left to right)
                    if (sweepAlpha > 0.01f) {
                        Canvas(modifier = Modifier.fillMaxSize()) {
                            val sweepX = size.width * (sweepProgress * 2.2f - 0.6f)
                            drawRect(
                                brush = Brush.horizontalGradient(
                                    colors = listOf(
                                        Color.Transparent,
                                        GlowWhite.copy(alpha = 0.40f * sweepAlpha),
                                        HighlightLavender.copy(alpha = 0.55f * sweepAlpha),
                                        Color.Transparent
                                    ),
                                    startX = sweepX - 50.dp.toPx(),
                                    endX = sweepX + 50.dp.toPx()
                                ),
                                blendMode = BlendMode.Screen
                            )
                        }
                    }
                }
            }

            Spacer(Modifier.height(20.dp))

            // Migration Paths & Connectivity Indicators (Device A ⇄ Device B)
            Box(
                modifier = Modifier
                    .width(emblemSize * 1.35f)
                    .height(24.dp)
                    .alpha(arrowsAlpha),
                contentAlignment = Alignment.Center
            ) {
                Canvas(modifier = Modifier.fillMaxSize()) {
                    val w = size.width
                    val h = size.height
                    val centerY = h * 0.5f

                    // Stream track
                    drawLine(
                        brush = Brush.horizontalGradient(
                            colors = listOf(
                                Color.Transparent,
                                MidnightViolet.copy(alpha = 0.45f),
                                ElectricViolet.copy(alpha = 0.60f),
                                MidnightViolet.copy(alpha = 0.45f),
                                Color.Transparent
                            )
                        ),
                        start = Offset(0f, centerY),
                        end = Offset(w, centerY),
                        strokeWidth = 2.dp.toPx(),
                        cap = StrokeCap.Round
                    )

                    // Traveling pulse light along migration path
                    val pulseCenter = w * ((arrowsProgress * 1.5f) % 1f)
                    drawCircle(
                        brush = Brush.radialGradient(
                            colors = listOf(
                                GlowWhite,
                                HighlightLavender,
                                Color.Transparent
                            ),
                            center = Offset(pulseCenter, centerY),
                            radius = 12.dp.toPx()
                        ),
                        center = Offset(pulseCenter, centerY),
                        radius = 12.dp.toPx()
                    )

                    // Left & Right Arrow Heads (Device A ⇄ Device B)
                    val arrowSize = 5.dp.toPx()
                    // Left Arrow (<)
                    val leftPath = Path().apply {
                        moveTo(10.dp.toPx() + arrowSize, centerY - arrowSize)
                        lineTo(10.dp.toPx(), centerY)
                        lineTo(10.dp.toPx() + arrowSize, centerY + arrowSize)
                    }
                    drawPath(
                        path = leftPath,
                        color = VividPurple.copy(alpha = arrowsAlpha),
                        style = Stroke(width = 2.dp.toPx(), cap = StrokeCap.Round)
                    )

                    // Right Arrow (>)
                    val rightPath = Path().apply {
                        moveTo(w - 10.dp.toPx() - arrowSize, centerY - arrowSize)
                        lineTo(w - 10.dp.toPx(), centerY)
                        lineTo(w - 10.dp.toPx() - arrowSize, centerY + arrowSize)
                    }
                    drawPath(
                        path = rightPath,
                        color = VividPurple.copy(alpha = arrowsAlpha),
                        style = Stroke(width = 2.dp.toPx(), cap = StrokeCap.Round)
                    )
                }

                // Sub-label for directional migration connectivity
                Row(
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(horizontal = 4.dp)
                ) {
                    Text(
                        text = "HOST",
                        color = HighlightLavender.copy(alpha = 0.85f),
                        fontSize = 8.sp,
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 1.sp
                    )
                    Text(
                        text = "MIGRATION",
                        color = GlowWhite.copy(alpha = 0.95f),
                        fontSize = 8.sp,
                        fontWeight = FontWeight.SemiBold,
                        letterSpacing = 1.2.sp
                    )
                    Text(
                        text = "CLIENT",
                        color = HighlightLavender.copy(alpha = 0.85f),
                        fontSize = 8.sp,
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 1.sp
                    )
                }
            }

            Spacer(Modifier.height(18.dp))

            // Typography: "SMART MIGRATE" with upward motion and clean font
            Column(
                modifier = Modifier
                    .offset(y = textOffsetY)
                    .alpha(textAlpha),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Text(
                    text = "SMART MIGRATE",
                    color = Color(0xFFF6F4FF),
                    fontSize = 21.sp,
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 3.5.sp,
                    fontFamily = FontFamily.SansSerif,
                    textAlign = TextAlign.Center
                )
                Spacer(Modifier.height(6.dp))
                Text(
                    text = "POWERED BY MIGROUTE",
                    color = HighlightLavender.copy(alpha = 0.80f),
                    fontSize = 9.sp,
                    fontWeight = FontWeight.Medium,
                    letterSpacing = 2.4.sp,
                    textAlign = TextAlign.Center
                )
            }
        }

        // 4. Subtle Bottom Security / Platform Pillar
        Box(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .padding(bottom = 28.dp)
                .alpha(textAlpha * 0.75f)
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(
                    modifier = Modifier
                        .size(5.dp)
                        .background(VividPurple, CircleShape)
                )
                Spacer(Modifier.width(7.dp))
                Text(
                    text = "SECURE CROSS-DEVICE CONNECTIVITY",
                    color = Color(0xFF8A84B0),
                    fontSize = 9.sp,
                    letterSpacing = 1.5.sp,
                    fontWeight = FontWeight.Medium
                )
            }
        }
    }
}
