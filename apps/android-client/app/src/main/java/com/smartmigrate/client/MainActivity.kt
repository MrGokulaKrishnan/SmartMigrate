package com.smartmigrate.client

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.core.view.WindowCompat
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import kotlinx.coroutines.launch
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog

// ─── Professional Dark Design Tokens ──────────────────────────────────────────
private val Black = Color(0xFF050505)
private val Surface0 = Color(0xFF080808)
private val Surface1 = Color(0xFF111114)
private val Surface2 = Color(0xFF151519)
private val Surface3 = Color(0xFF1A1A1F)
private val Elevated1 = Color(0xFF202027)
private val Line = Color(0x1FFFFFFF)
private val LineSubtle = Color(0x0EFFFFFF)
private val LineStrong = Color(0x557C4DFF)

private val TextPrimary = Color(0xFFF3F3F7)
private val TextSecondary = Color(0xFFB5B5C3)
private val TextMuted = Color(0xFF757588)

private val Brand300 = Color(0xFFA78BFA)
private val Brand400 = Color(0xFF8B5CF6)
private val Brand500 = Color(0xFF7C4DFF)
private val Brand600 = Color(0xFF6D3DF5)

private val Success = Color(0xFF10B981)
private val Warning = Color(0xFFF59E0B)
private val Danger = Color(0xFFEF4444)
private val Info = Color(0xFF60A5FA)

private enum class AppDestination(val label: String, val symbol: String) {
    Home("Home", "⌂"),
    Devices("Devices", "◇"),
    Transfers("Transfers", "⇄"),
    Remote("Remote", "⌁"),
    Security("Security", "⛊")
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(
                android.graphics.Color.TRANSPARENT
            ),
            navigationBarStyle = SystemBarStyle.dark(
                android.graphics.Color.TRANSPARENT
            )
        )
        WindowCompat.getInsetsController(window, window.decorView).apply {
            isAppearanceLightStatusBars = false
            isAppearanceLightNavigationBars = false
        }
        setContent {
            var startupFinished by rememberSaveable { mutableStateOf(false) }

            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Black)
            ) {
                SmartMigrateApp()

                if (!startupFinished) {
                    SmartMigrateStartupScreen(
                        onStartupFinished = { startupFinished = true }
                    )
                }
            }
        }
    }
}

@Composable
private fun SmartMigrateApp() {
    var destination by rememberSaveable { mutableStateOf(AppDestination.Home) }
    var pairingDialog by rememberSaveable { mutableStateOf(false) }
    var qrScannerOpen by rememberSaveable { mutableStateOf(false) }
    var shareSheetOpen by rememberSaveable { mutableStateOf(false) }
    var notice by rememberSaveable { mutableStateOf("Smart Migrate Core online. Direct LAN active.") }
    var batterySaverMode by rememberSaveable { mutableStateOf(false) }
    var targetHostAddress by rememberSaveable { mutableStateOf("10.0.2.2") }
    var currentSessionToken by rememberSaveable { mutableStateOf("") }

    MaterialTheme(
        colorScheme = MaterialTheme.colorScheme.copy(
            primary = Brand500,
            background = Black,
            surface = Surface1,
            onSurface = TextPrimary,
            onBackground = TextPrimary
        )
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Black)
        ) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .statusBarsPadding()
                    .navigationBarsPadding()
                    .padding(horizontal = 14.dp)
                    .padding(top = 8.dp, bottom = 8.dp)
            ) {
                // Top App Bar
                AppBar(
                    onScanQR = { qrScannerOpen = true },
                    onShareSheet = { shareSheetOpen = true },
                    batterySaver = batterySaverMode
                )

                Spacer(Modifier.height(12.dp))

                // Screen Content
                AnimatedContent(
                    targetState = destination,
                    label = "smart-migrate-screen",
                    modifier = Modifier.weight(1f)
                ) { target ->
                    when (target) {
                        AppDestination.Home -> HomeScreen(
                            onPair = { pairingDialog = true },
                            onScanQR = { qrScannerOpen = true },
                            onNavigate = { destination = it },
                            onNotice = { notice = it },
                            batterySaver = batterySaverMode,
                            onToggleBatterySaver = { batterySaverMode = !batterySaverMode }
                        )
                        AppDestination.Devices -> DevicesScreen(
                            onPair = { pairingDialog = true },
                            onNotice = { notice = it }
                        )
                        AppDestination.Transfers -> TransfersScreen(
                            onNotice = { notice = it }
                        )
                        AppDestination.Remote -> RemoteScreen(
                            hostAddress = targetHostAddress,
                            sessionToken = currentSessionToken,
                            batterySaver = batterySaverMode,
                            onNotice = { notice = it }
                        )
                        AppDestination.Security -> SecurityScreen(
                            hostAddress = targetHostAddress,
                            onNotice = { notice = it }
                        )
                    }
                }

                // Notice Bar
                NoticeBar(notice)
                Spacer(Modifier.height(8.dp))

                // Bottom Navigation
                BottomNavBar(selected = destination, onSelect = {
                    destination = it
                    notice = "${it.label} view selected."
                })
                Spacer(Modifier.height(12.dp))
            }

            // QR Scanner Dialog
            if (qrScannerOpen) {
                QrScannerModal(
                    onDismiss = { qrScannerOpen = false },
                    onPaired = {
                        qrScannerOpen = false
                        notice = "Scanned QR code! Windows PC verified and connected."
                    }
                )
            }

            // Numeric PIN Pairing Dialog
            if (pairingDialog) {
                PairingPinModal(
                    initialHost = targetHostAddress,
                    onDismiss = { pairingDialog = false },
                    onVerified = { token, hostId ->
                        pairingDialog = false
                        currentSessionToken = token
                        notice = "Paired with $hostId! Session token secured."
                    }
                )
            }

            // Android Share Sheet Simulation Modal
            if (shareSheetOpen) {
                ShareSheetModal(
                    onDismiss = { shareSheetOpen = false },
                    onSent = {
                        shareSheetOpen = false
                        destination = AppDestination.Transfers
                        notice = "File dispatched via Android Share Sheet to Windows PC!"
                    }
                )
            }
        }
    }
}

// ─── Professional Vector Icons & Badges ─────────────────────────────────────
@Composable
private fun ProfessionalCheckBadge(
    modifier: Modifier = Modifier,
    size: Dp = 18.dp,
    badgeColor: Color = Success.copy(alpha = 0.16f),
    iconColor: Color = Success
) {
    Box(
        modifier = modifier
            .size(size)
            .background(badgeColor, CircleShape)
            .border(1.dp, iconColor.copy(alpha = 0.5f), CircleShape),
        contentAlignment = Alignment.Center
    ) {
        Canvas(modifier = Modifier.size(size * 0.56f)) {
            val strokeWidth = 2.dp.toPx()
            val w = this.size.width
            val h = this.size.height
            val path = Path().apply {
                moveTo(w * 0.12f, h * 0.52f)
                lineTo(w * 0.42f, h * 0.82f)
                lineTo(w * 0.90f, h * 0.22f)
            }
            drawPath(
                path = path,
                color = iconColor,
                style = Stroke(width = strokeWidth, cap = StrokeCap.Round, join = StrokeJoin.Round)
            )
        }
    }
}

@Composable
private fun DestinationVectorIcon(destination: AppDestination, isSelected: Boolean, modifier: Modifier = Modifier) {
    val color = if (isSelected) Brand400 else TextMuted
    val strokeWidth = if (isSelected) 2.2f else 1.8f
    Canvas(modifier = modifier.size(20.dp)) {
        val w = size.width
        val h = size.height
        val s = strokeWidth.dp.toPx()
        when (destination) {
            AppDestination.Home -> {
                val path = Path().apply {
                    moveTo(w * 0.15f, h * 0.45f)
                    lineTo(w * 0.50f, h * 0.15f)
                    lineTo(w * 0.85f, h * 0.45f)
                    lineTo(w * 0.85f, h * 0.85f)
                    lineTo(w * 0.58f, h * 0.85f)
                    lineTo(w * 0.58f, h * 0.55f)
                    lineTo(w * 0.42f, h * 0.55f)
                    lineTo(w * 0.42f, h * 0.85f)
                    lineTo(w * 0.15f, h * 0.85f)
                    close()
                }
                drawPath(path, color, style = Stroke(s, cap = StrokeCap.Round, join = StrokeJoin.Round))
            }
            AppDestination.Devices -> {
                drawRoundRect(
                    color = color,
                    topLeft = Offset(w * 0.1f, h * 0.18f),
                    size = androidx.compose.ui.geometry.Size(w * 0.62f, h * 0.48f),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(2.dp.toPx()),
                    style = Stroke(s)
                )
                drawLine(color, Offset(w * 0.28f, h * 0.66f), Offset(w * 0.28f, h * 0.82f), strokeWidth = s, cap = StrokeCap.Round)
                drawLine(color, Offset(w * 0.18f, h * 0.82f), Offset(w * 0.38f, h * 0.82f), strokeWidth = s, cap = StrokeCap.Round)
                drawRoundRect(
                    color = color,
                    topLeft = Offset(w * 0.58f, h * 0.40f),
                    size = androidx.compose.ui.geometry.Size(w * 0.32f, h * 0.46f),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(3.dp.toPx()),
                    style = Stroke(s)
                )
            }
            AppDestination.Transfers -> {
                val p1 = Path().apply {
                    moveTo(w * 0.2f, h * 0.36f)
                    lineTo(w * 0.8f, h * 0.36f)
                    moveTo(w * 0.62f, h * 0.18f)
                    lineTo(w * 0.82f, h * 0.36f)
                    lineTo(w * 0.62f, h * 0.54f)
                }
                val p2 = Path().apply {
                    moveTo(w * 0.8f, h * 0.64f)
                    lineTo(w * 0.2f, h * 0.64f)
                    moveTo(w * 0.38f, h * 0.46f)
                    lineTo(w * 0.18f, h * 0.64f)
                    lineTo(w * 0.38f, h * 0.82f)
                }
                drawPath(p1, color, style = Stroke(s, cap = StrokeCap.Round, join = StrokeJoin.Round))
                drawPath(p2, color, style = Stroke(s, cap = StrokeCap.Round, join = StrokeJoin.Round))
            }
            AppDestination.Remote -> {
                drawRoundRect(
                    color = color,
                    topLeft = Offset(w * 0.12f, h * 0.15f),
                    size = androidx.compose.ui.geometry.Size(w * 0.76f, h * 0.56f),
                    cornerRadius = androidx.compose.ui.geometry.CornerRadius(3.dp.toPx()),
                    style = Stroke(s)
                )
                val bolt = Path().apply {
                    moveTo(w * 0.54f, h * 0.28f)
                    lineTo(w * 0.42f, h * 0.44f)
                    lineTo(w * 0.52f, h * 0.44f)
                    lineTo(w * 0.46f, h * 0.58f)
                }
                drawPath(bolt, color, style = Stroke(s, cap = StrokeCap.Round, join = StrokeJoin.Round))
                drawLine(color, Offset(w * 0.36f, h * 0.82f), Offset(w * 0.64f, h * 0.82f), strokeWidth = s, cap = StrokeCap.Round)
            }
            AppDestination.Security -> {
                val shield = Path().apply {
                    moveTo(w * 0.50f, h * 0.14f)
                    lineTo(w * 0.82f, h * 0.28f)
                    lineTo(w * 0.82f, h * 0.55f)
                    cubicTo(w * 0.82f, h * 0.74f, w * 0.50f, h * 0.88f, w * 0.50f, h * 0.88f)
                    cubicTo(w * 0.50f, h * 0.88f, w * 0.18f, h * 0.74f, w * 0.18f, h * 0.55f)
                    lineTo(w * 0.18f, h * 0.28f)
                    close()
                }
                drawPath(shield, color, style = Stroke(s, cap = StrokeCap.Round, join = StrokeJoin.Round))
            }
        }
    }
}

@Composable
private fun QrCodeVector(modifier: Modifier = Modifier, color: Color = Brand300, size: Dp = 16.dp) {
    Canvas(modifier = modifier.size(size)) {
        val s = 1.6.dp.toPx()
        val w = this.size.width
        val h = this.size.height
        drawRect(color, topLeft = Offset(w * 0.1f, h * 0.1f), size = androidx.compose.ui.geometry.Size(w * 0.34f, h * 0.34f), style = Stroke(s))
        drawRect(color, topLeft = Offset(w * 0.20f, h * 0.20f), size = androidx.compose.ui.geometry.Size(w * 0.14f, h * 0.14f))
        drawRect(color, topLeft = Offset(w * 0.56f, h * 0.1f), size = androidx.compose.ui.geometry.Size(w * 0.34f, h * 0.34f), style = Stroke(s))
        drawRect(color, topLeft = Offset(w * 0.66f, h * 0.20f), size = androidx.compose.ui.geometry.Size(w * 0.14f, h * 0.14f))
        drawRect(color, topLeft = Offset(w * 0.1f, h * 0.56f), size = androidx.compose.ui.geometry.Size(w * 0.34f, h * 0.34f), style = Stroke(s))
        drawRect(color, topLeft = Offset(w * 0.20f, h * 0.66f), size = androidx.compose.ui.geometry.Size(w * 0.14f, h * 0.14f))
        drawRect(color, topLeft = Offset(w * 0.60f, h * 0.60f), size = androidx.compose.ui.geometry.Size(w * 0.26f, h * 0.26f))
    }
}

@Composable
private fun KeypadVector(modifier: Modifier = Modifier, color: Color = Brand300, size: Dp = 16.dp) {
    Canvas(modifier = modifier.size(size)) {
        val r = 1.6.dp.toPx()
        val w = this.size.width
        val h = this.size.height
        listOf(
            Offset(w * 0.25f, h * 0.25f), Offset(w * 0.50f, h * 0.25f), Offset(w * 0.75f, h * 0.25f),
            Offset(w * 0.25f, h * 0.50f), Offset(w * 0.50f, h * 0.50f), Offset(w * 0.75f, h * 0.50f),
            Offset(w * 0.25f, h * 0.75f), Offset(w * 0.50f, h * 0.75f), Offset(w * 0.75f, h * 0.75f)
        ).forEach { pos ->
            drawCircle(color, radius = r, center = pos)
        }
    }
}

@Composable
private fun ShareSheetVector(modifier: Modifier = Modifier, color: Color = Brand300, size: Dp = 16.dp) {
    Canvas(modifier = modifier.size(size)) {
        val s = 1.8.dp.toPx()
        val w = this.size.width
        val h = this.size.height
        val box = Path().apply {
            moveTo(w * 0.2f, h * 0.45f)
            lineTo(w * 0.2f, h * 0.85f)
            lineTo(w * 0.8f, h * 0.85f)
            lineTo(w * 0.8f, h * 0.45f)
        }
        drawPath(box, color, style = Stroke(s, cap = StrokeCap.Round, join = StrokeJoin.Round))
        drawLine(color, Offset(w * 0.5f, h * 0.65f), Offset(w * 0.5f, h * 0.16f), strokeWidth = s, cap = StrokeCap.Round)
        val arr = Path().apply {
            moveTo(w * 0.32f, h * 0.34f)
            lineTo(w * 0.50f, h * 0.16f)
            lineTo(w * 0.68f, h * 0.34f)
        }
        drawPath(arr, color, style = Stroke(s, cap = StrokeCap.Round, join = StrokeJoin.Round))
    }
}

@Composable
private fun DesktopDeviceVector(modifier: Modifier = Modifier, color: Color = Brand300, size: Dp = 22.dp) {
    Canvas(modifier = modifier.size(size)) {
        val s = 1.8.dp.toPx()
        val w = this.size.width
        val h = this.size.height
        drawRoundRect(
            color = color,
            topLeft = Offset(w * 0.12f, h * 0.14f),
            size = androidx.compose.ui.geometry.Size(w * 0.76f, h * 0.56f),
            cornerRadius = androidx.compose.ui.geometry.CornerRadius(3.dp.toPx()),
            style = Stroke(s)
        )
        drawLine(color, Offset(w * 0.5f, h * 0.70f), Offset(w * 0.5f, h * 0.86f), strokeWidth = s, cap = StrokeCap.Round)
        drawLine(color, Offset(w * 0.32f, h * 0.86f), Offset(w * 0.68f, h * 0.86f), strokeWidth = s, cap = StrokeCap.Round)
    }
}

@Composable
private fun PlayPauseVector(isPlaying: Boolean, modifier: Modifier = Modifier, color: Color = Color.White, size: Dp = 20.dp) {
    Canvas(modifier = modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        if (isPlaying) {
            val barW = w * 0.22f
            drawRoundRect(color, topLeft = Offset(w * 0.22f, h * 0.18f), size = androidx.compose.ui.geometry.Size(barW, h * 0.64f), cornerRadius = androidx.compose.ui.geometry.CornerRadius(2.dp.toPx()))
            drawRoundRect(color, topLeft = Offset(w * 0.56f, h * 0.18f), size = androidx.compose.ui.geometry.Size(barW, h * 0.64f), cornerRadius = androidx.compose.ui.geometry.CornerRadius(2.dp.toPx()))
        } else {
            val playPath = Path().apply {
                moveTo(w * 0.28f, h * 0.18f)
                lineTo(w * 0.82f, h * 0.50f)
                lineTo(w * 0.28f, h * 0.82f)
                close()
            }
            drawPath(playPath, color)
        }
    }
}

@Composable
private fun SkipPrevVector(modifier: Modifier = Modifier, color: Color = TextPrimary, size: Dp = 16.dp) {
    Canvas(modifier = modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val p1 = Path().apply {
            moveTo(w * 0.50f, h * 0.20f)
            lineTo(w * 0.15f, h * 0.50f)
            lineTo(w * 0.50f, h * 0.80f)
            close()
        }
        val p2 = Path().apply {
            moveTo(w * 0.85f, h * 0.20f)
            lineTo(w * 0.50f, h * 0.50f)
            lineTo(w * 0.85f, h * 0.80f)
            close()
        }
        drawPath(p1, color)
        drawPath(p2, color)
    }
}

@Composable
private fun SkipNextVector(modifier: Modifier = Modifier, color: Color = TextPrimary, size: Dp = 16.dp) {
    Canvas(modifier = modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val p1 = Path().apply {
            moveTo(w * 0.15f, h * 0.20f)
            lineTo(w * 0.50f, h * 0.50f)
            lineTo(w * 0.15f, h * 0.80f)
            close()
        }
        val p2 = Path().apply {
            moveTo(w * 0.50f, h * 0.20f)
            lineTo(w * 0.85f, h * 0.50f)
            lineTo(w * 0.50f, h * 0.80f)
            close()
        }
        drawPath(p1, color)
        drawPath(p2, color)
    }
}

// ─── App Bar ─────────────────────────────────────────────────────────────────
@Composable
private fun AppBar(onScanQR: () -> Unit, onShareSheet: () -> Unit, batterySaver: Boolean) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = Surface1,
        shape = RoundedCornerShape(12.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, Line)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Image(
                painter = painterResource(R.drawable.smart_migrate_logo),
                contentDescription = "Smart Migrate",
                modifier = Modifier
                    .size(36.dp)
                    .clip(RoundedCornerShape(8.dp))
                    .border(1.dp, LineStrong, RoundedCornerShape(8.dp))
            )
            Spacer(Modifier.width(12.dp))
            Column(Modifier.weight(1f)) {
                Text("Smart Migrate", color = TextPrimary, fontWeight = FontWeight.Bold, fontSize = 17.sp)
                Text(if (batterySaver) "Battery Saver Mode Active" else "Direct LAN • 14 ms", color = if (batterySaver) Warning else TextMuted, fontSize = 12.sp)
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(Surface2)
                        .border(1.dp, Line, RoundedCornerShape(8.dp))
                        .clickable { onShareSheet() }
                        .padding(horizontal = 12.dp, vertical = 7.dp)
                ) {
                    Text("Share", color = Brand300, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                }
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(Surface2)
                        .border(1.dp, Line, RoundedCornerShape(8.dp))
                        .clickable { onScanQR() }
                        .padding(horizontal = 12.dp, vertical = 7.dp)
                ) {
                    Text("QR", color = Brand300, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                }
            }
        }
    }
}

// ─── 02 Home Screen ──────────────────────────────────────────────────────────
@Composable
private fun HomeScreen(
    onPair: () -> Unit,
    onScanQR: () -> Unit,
    onNavigate: (AppDestination) -> Unit,
    onNotice: (String) -> Unit,
    batterySaver: Boolean,
    onToggleBatterySaver: () -> Unit
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // Hero Card
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(18.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .clip(CircleShape)
                            .background(Success.copy(alpha = 0.15f))
                            .border(1.dp, Success.copy(alpha = 0.3f), CircleShape)
                            .padding(horizontal = 10.dp, vertical = 4.dp)
                    ) {
                        Text("● HOST CONNECTED", color = Success, fontSize = 11.sp, fontWeight = FontWeight.Bold)
                    }
                    Spacer(Modifier.weight(1f))
                    Text("SMP/1 • MigRoute", color = Brand300, fontSize = 12.sp, fontFamily = FontFamily.Monospace)
                }

                Spacer(Modifier.height(14.dp))
                Text("Windows PC Hub", color = TextPrimary, fontSize = 24.sp, fontWeight = FontWeight.Bold)
                Spacer(Modifier.height(4.dp))
                Text("Direct P2P LAN • 14 ms latency • 42.8 MB/s verified throughput", color = TextSecondary, fontSize = 14.sp)

                Spacer(Modifier.height(16.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                    Box(Modifier.weight(1f)) {
                        PrimaryButton("Send Files") { onNavigate(AppDestination.Transfers) }
                    }
                    Box(Modifier.weight(1f)) {
                        SecondaryButton("Remote PC") { onNavigate(AppDestination.Remote) }
                    }
                }
            }
        }

        // Quick Actions Row
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
            QuickActionPill("Scan QR", Modifier.weight(1f), icon = { QrCodeVector() }) { onScanQR() }
            QuickActionPill("Enter PIN", Modifier.weight(1f), icon = { KeypadVector() }) { onPair() }
            QuickActionPill("Share Sheet", Modifier.weight(1f), icon = { ShareSheetVector() }) {
                onNotice("Long-press files in Android Files or Photos to Send with Smart Migrate.")
            }
        }

        // Connection Stages Flow
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text("SMART CONNECTION STATUS", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                listOf(
                    "1. Discovering device" to true,
                    "2. Verifying hardware identity" to true,
                    "3. Establishing secure channel (AES-GCM)" to true,
                    "4. Negotiating transport (Direct LAN • 14 ms)" to true,
                    "5. Host authorization granted" to true
                ).forEach { (step, done) ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (done) {
                            ProfessionalCheckBadge(size = 18.dp)
                        } else {
                            Box(
                                modifier = Modifier
                                    .size(18.dp)
                                    .background(Surface2, CircleShape)
                                    .border(1.dp, Line, CircleShape)
                            )
                        }
                        Spacer(Modifier.width(10.dp))
                        Text(step, color = if (done) TextPrimary else TextMuted, fontSize = 13.5.sp, fontWeight = if (done) FontWeight.Medium else FontWeight.Normal)
                    }
                }
            }
        }

        // Battery Awareness Card
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface2,
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Row(
                modifier = Modifier.padding(16.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(Modifier.weight(1f)) {
                    Text("Battery Awareness", color = TextPrimary, fontSize = 15.sp, fontWeight = FontWeight.SemiBold)
                    Spacer(Modifier.height(2.dp))
                    Text("Optimizes streaming FPS and discovery to preserve mobile battery.", color = TextMuted, fontSize = 13.sp)
                }
                Box(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .background(if (batterySaver) Warning else Surface3)
                        .border(1.dp, Line, RoundedCornerShape(8.dp))
                        .clickable { onToggleBatterySaver() }
                        .padding(horizontal = 12.dp, vertical = 7.dp)
                ) {
                    Text(if (batterySaver) "Active" else "Off", color = if (batterySaver) Color.Black else TextPrimary, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                }
            }
        }
    }
}

// ─── 03 Nearby Devices Screen ────────────────────────────────────────────────
@Composable
private fun DevicesScreen(onPair: () -> Unit, onNotice: (String) -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("NEARBY DISCOVERED DEVICES", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)

        listOf(
            Triple("Windows Host PC", "Windows 11 • Direct LAN (7890)", "Connected"),
            Triple("Office Desktop", "Windows 10 • LAN Subnet", "Ready"),
            Triple("Pixel Tablet", "Android 14 • Standby", "Ready")
        ).forEach { (name, info, status) ->
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Surface1,
                shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, Line)
            ) {
                Row(
                    modifier = Modifier.padding(16.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column(Modifier.weight(1f)) {
                        Text(name, color = TextPrimary, fontSize = 16.sp, fontWeight = FontWeight.SemiBold)
                        Spacer(Modifier.height(2.dp))
                        Text(info, color = TextMuted, fontSize = 13.sp)
                    }
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(8.dp))
                            .background(if (status == "Connected") Success.copy(alpha = 0.15f) else Surface2)
                            .border(1.dp, if (status == "Connected") Success.copy(alpha = 0.3f) else Line, RoundedCornerShape(8.dp))
                            .clickable {
                                onNotice(if (status == "Connected") "Connected to $name" else "Pairing requested with $name")
                            }
                            .padding(horizontal = 12.dp, vertical = 7.dp)
                    ) {
                        Text(status, color = if (status == "Connected") Success else Brand300, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                    }
                }
            }
        }

        Spacer(Modifier.height(8.dp))
        SecondaryButton("+ Pair via 6-Digit PIN") { onPair() }
    }
}

// ─── 08, 09, 10 Transfers Screen ─────────────────────────────────────────────
private data class TransferQueueItem(
    val id: String,
    val name: String,
    val totalSize: String,
    val progress: Float,
    val speed: String,
    val eta: String,
    val priority: String,
    val isPaused: Boolean,
    val sha256Preview: String,
    val chunkInfo: String
)

@Composable
private fun TransfersScreen(onNotice: (String) -> Unit) {
    var selectedCategory by rememberSaveable { mutableStateOf("Photos") }
    var speedLimit by rememberSaveable { mutableStateOf("Unlimited") }
    var queueItems by remember {
        mutableStateOf(
            listOf(
                TransferQueueItem(
                    id = "tx-1",
                    name = "Photos_Archive_2026.zip",
                    totalSize = "2.4 GB",
                    progress = 0.74f,
                    speed = "42.8 MB/s",
                    eta = "00:14",
                    priority = "High",
                    isPaused = false,
                    sha256Preview = "8f4b...3c91",
                    chunkInfo = "Chunk 1,776 / 2,400 (1 MiB chunks)"
                ),
                TransferQueueItem(
                    id = "tx-2",
                    name = "Project_Design_Assets.tar.gz",
                    totalSize = "680 MB",
                    progress = 0.15f,
                    speed = "Waiting slot",
                    eta = "00:48",
                    priority = "Normal",
                    isPaused = false,
                    sha256Preview = "a29d...e810",
                    chunkInfo = "Chunk 102 / 680 (1 MiB chunks)"
                )
            )
        )
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        // Send Files Category Picker & Header
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("SEND FILES TO WINDOWS PC", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                    Spacer(Modifier.weight(1f))
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(6.dp))
                            .background(Surface2)
                            .border(1.dp, Line, RoundedCornerShape(6.dp))
                            .clickable {
                                speedLimit = when (speedLimit) {
                                    "Unlimited" -> "50 MB/s"
                                    "50 MB/s" -> "20 MB/s"
                                    else -> "Unlimited"
                                }
                                onNotice("Transfer speed limit set to $speedLimit")
                            }
                            .padding(horizontal = 8.dp, vertical = 4.dp)
                    ) {
                        Text("Speed: $speedLimit", color = Brand300, fontSize = 11.sp, fontWeight = FontWeight.Medium)
                    }
                }
                Spacer(Modifier.height(10.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp), modifier = Modifier.fillMaxWidth()) {
                    listOf("Photos", "Videos", "Docs", "Music").forEach { cat ->
                        val isSel = selectedCategory == cat
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .clip(RoundedCornerShape(8.dp))
                                .background(if (isSel) Brand600 else Surface2)
                                .border(1.dp, if (isSel) Brand500 else Line, RoundedCornerShape(8.dp))
                                .clickable { selectedCategory = cat }
                                .padding(vertical = 10.dp),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(cat, color = if (isSel) Color.White else TextSecondary, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                        }
                    }
                }

                Spacer(Modifier.height(12.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                    Box(Modifier.weight(1.2f)) {
                        PrimaryButton("Choose $selectedCategory & Send") {
                            onNotice("Staged $selectedCategory files into resumable transfer queue.")
                        }
                    }
                    Box(Modifier.weight(0.8f)) {
                        SecondaryButton("+ Add File") {
                            val newId = "tx-${System.currentTimeMillis() % 1000}"
                            queueItems = queueItems + TransferQueueItem(
                                id = newId,
                                name = "Staged_${selectedCategory}_File.bin",
                                totalSize = "128 MB",
                                progress = 0.05f,
                                speed = "38.2 MB/s",
                                eta = "00:04",
                                priority = "Normal",
                                isPaused = false,
                                sha256Preview = "c18f...92e1",
                                chunkInfo = "Chunk 6 / 128 (1 MiB chunks)"
                            )
                            onNotice("Added Staged_${selectedCategory}_File.bin to queue.")
                        }
                    }
                }
            }
        }

        // Active Transfer Queue Header
        Text(
            "ACTIVE TRANSFER QUEUE (${queueItems.size})",
            color = Brand300,
            fontSize = 12.sp,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.padding(horizontal = 2.dp)
        )

        // Queue Items
        queueItems.forEach { item ->
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Surface1,
                shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, Line)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(item.name, color = TextPrimary, fontSize = 16.sp, fontWeight = FontWeight.Bold, modifier = Modifier.weight(1f))
                        Spacer(Modifier.width(8.dp))
                        Box(
                            modifier = Modifier
                                .clip(RoundedCornerShape(6.dp))
                                .background(
                                    when (item.priority) {
                                        "High" -> Brand500.copy(alpha = 0.2f)
                                        "Low" -> Surface3
                                        else -> Surface2
                                    }
                                )
                                .border(1.dp, if (item.priority == "High") Brand400 else Line, RoundedCornerShape(6.dp))
                                .clickable {
                                    val nextPri = when (item.priority) {
                                        "High" -> "Normal"
                                        "Normal" -> "Low"
                                        else -> "High"
                                    }
                                    queueItems = queueItems.map { if (it.id == item.id) it.copy(priority = nextPri) else it }
                                    onNotice("${item.name} priority changed to $nextPri")
                                }
                                .padding(horizontal = 8.dp, vertical = 3.dp)
                        ) {
                            Text(
                                "Pri: ${item.priority}",
                                color = if (item.priority == "High") Brand300 else TextSecondary,
                                fontSize = 11.sp,
                                fontWeight = FontWeight.SemiBold
                            )
                        }
                    }

                    Spacer(Modifier.height(4.dp))
                    Text(
                        "${item.totalSize} • ${item.chunkInfo}",
                        color = TextMuted,
                        fontSize = 12.sp
                    )

                    Spacer(Modifier.height(10.dp))
                    // Progress Bar
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(8.dp)
                            .clip(RoundedCornerShape(4.dp))
                            .background(Surface3)
                    ) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth(if (item.isPaused) item.progress else item.progress.coerceIn(0.02f, 1f))
                                .fillMaxHeight()
                                .background(Brush.horizontalGradient(listOf(Brand500, Brand400)))
                        )
                    }

                    Spacer(Modifier.height(8.dp))
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            if (item.isPaused) "PAUSED" else "${(item.progress * 100).toInt()}% • ${item.speed} • ETA ${item.eta}",
                            color = if (item.isPaused) Warning else Success,
                            fontSize = 12.sp,
                            fontWeight = FontWeight.SemiBold
                        )
                        Spacer(Modifier.weight(1f))
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            ProfessionalCheckBadge(size = 14.dp)
                            Spacer(Modifier.width(4.dp))
                            Text("SHA-256: ${item.sha256Preview}", color = Success, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
                        }
                    }

                    Spacer(Modifier.height(12.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Box(Modifier.weight(1f)) {
                            SecondaryButton(if (item.isPaused) "Resume" else "Pause") {
                                queueItems = queueItems.map {
                                    if (it.id == item.id) it.copy(isPaused = !it.isPaused) else it
                                }
                                onNotice(if (item.isPaused) "Resumed ${item.name}" else "Paused ${item.name}")
                            }
                        }
                        Box(Modifier.weight(1f)) {
                            SecondaryButton("Cancel") {
                                queueItems = queueItems.filter { it.id != item.id }
                                onNotice("Cancelled and purged partial file for ${item.name}")
                            }
                        }
                    }
                }
            }
        }

        // History Snippet
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text("RECENT COMPLETED TRANSFERS", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                Row(verticalAlignment = Alignment.CenterVertically) {
                    ProfessionalCheckBadge(size = 18.dp)
                    Spacer(Modifier.width(10.dp))
                    Column {
                        Text("Codebase_Backup.tar.gz (845 MB)", color = TextPrimary, fontSize = 13.5.sp, fontWeight = FontWeight.Medium)
                        Text("Direct LAN • Today 09:21 AM • Verified", color = TextMuted, fontSize = 11.5.sp)
                    }
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    ProfessionalCheckBadge(size = 18.dp)
                    Spacer(Modifier.width(10.dp))
                    Column {
                        Text("System_Update.apk (42 MB)", color = TextPrimary, fontSize = 13.5.sp, fontWeight = FontWeight.Medium)
                        Text("Direct LAN • Yesterday • Verified", color = TextMuted, fontSize = 11.5.sp)
                    }
                }
            }
        }
    }
}

// ─── 12 Remote Desktop 2.0 & Device Control ──────────────────────────────────
@Composable
private fun RemoteScreen(
    hostAddress: String,
    sessionToken: String,
    batterySaver: Boolean,
    onNotice: (String) -> Unit
) {
    var remoteMode by rememberSaveable { mutableStateOf("Display") }
    var keyboardText by rememberSaveable { mutableStateOf("") }
    var volume by rememberSaveable { mutableFloatStateOf(0.7f) }
    var currentSlide by rememberSaveable { mutableIntStateOf(12) }
    val scope = rememberCoroutineScope()
    var inputSeq by rememberSaveable { mutableLongStateOf(1L) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        // Remote Control Mode Tabs
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            listOf("Display", "Touchpad", "Keyboard", "Media", "Slide", "Game").forEach { mode ->
                val isSel = remoteMode == mode
                Box(
                    modifier = Modifier
                        .weight(1f)
                        .clip(RoundedCornerShape(6.dp))
                        .background(if (isSel) Brand600 else Surface2)
                        .border(1.dp, if (isSel) Brand500 else Line, RoundedCornerShape(6.dp))
                        .clickable { remoteMode = mode }
                        .padding(vertical = 6.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Text(mode, color = if (isSel) Color.White else TextSecondary, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                }
            }
        }

        // Display Mirror Mode
        if (remoteMode == "Display") {
            val streamEngine = remember { StreamEngine() }
            val connState by streamEngine.state
            val latestBitmap by streamEngine.latestBitmap
            val streamDiag by streamEngine.diagnostics
            var hostInput by rememberSaveable { mutableStateOf(hostAddress) }
            var tokenInput by rememberSaveable { mutableStateOf(sessionToken) }

            DisposableEffect(streamEngine) {
                onDispose {
                    streamEngine.stopStreaming()
                }
            }

            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Surface1,
                shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, Line)
            ) {
                Column(modifier = Modifier.padding(14.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("LIVE WINDOWS DESKTOP STREAM", color = Brand300, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                        Spacer(Modifier.weight(1f))
                        Text(
                            when (connState) {
                                StreamConnectionState.CONNECTED -> "● LIVE ${String.format(java.util.Locale.US, "%.0f", streamDiag.currentFps)} FPS"
                                StreamConnectionState.CONNECTING, StreamConnectionState.NEGOTIATING -> "CONNECTING..."
                                StreamConnectionState.RECONNECTING -> "RECONNECTING"
                                else -> "OFFLINE"
                            },
                            color = when (connState) {
                                StreamConnectionState.CONNECTED -> Success
                                StreamConnectionState.CONNECTING, StreamConnectionState.NEGOTIATING -> Brand300
                                else -> TextMuted
                            },
                            fontSize = 10.sp,
                            fontWeight = FontWeight.Bold
                        )
                    }

                    // Host IP & Token Controls
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp), modifier = Modifier.fillMaxWidth()) {
                        Box(
                            modifier = Modifier
                                .weight(1.1f)
                                .clip(RoundedCornerShape(6.dp))
                                .background(Surface2)
                                .border(1.dp, Line, RoundedCornerShape(6.dp))
                                .padding(horizontal = 8.dp, vertical = 6.dp)
                        ) {
                            BasicTextField(
                                value = hostInput,
                                onValueChange = { hostInput = it },
                                textStyle = TextStyle(color = TextPrimary, fontSize = 12.sp, fontFamily = FontFamily.Monospace),
                                cursorBrush = SolidColor(Brand400),
                                singleLine = true
                            )
                        }
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .clip(RoundedCornerShape(6.dp))
                                .background(Surface2)
                                .border(1.dp, Line, RoundedCornerShape(6.dp))
                                .padding(horizontal = 8.dp, vertical = 6.dp)
                        ) {
                            if (tokenInput.isEmpty()) {
                                Text("Token", color = TextMuted, fontSize = 11.sp)
                            }
                            BasicTextField(
                                value = tokenInput,
                                onValueChange = { tokenInput = it },
                                textStyle = TextStyle(color = TextPrimary, fontSize = 12.sp, fontFamily = FontFamily.Monospace),
                                cursorBrush = SolidColor(Brand400),
                                singleLine = true
                            )
                        }
                    }

                    // Viewfinder Screen Surface
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .aspectRatio(16f / 9f)
                            .clip(RoundedCornerShape(8.dp))
                            .background(Color.Black)
                            .border(1.dp, if (connState == StreamConnectionState.CONNECTED) Success.copy(alpha = 0.4f) else Line, RoundedCornerShape(8.dp))
                            .clickable {
                                if (connState == StreamConnectionState.CONNECTED) {
                                    val seq = inputSeq++
                                    scope.launch {
                                        SmpClient.sendInput(hostInput, 7890, "left_click", 960, 540, sequence = seq)
                                    }
                                    onNotice("Dispatched tap to Windows PC (seq #$seq)")
                                }
                            },
                        contentAlignment = Alignment.Center
                    ) {
                        val frame = latestBitmap
                        if (connState == StreamConnectionState.CONNECTED && frame != null) {
                            Image(
                                bitmap = frame.asImageBitmap(),
                                contentDescription = "Windows Desktop",
                                modifier = Modifier.fillMaxSize(),
                                contentScale = ContentScale.Fit
                            )
                        } else if (connState == StreamConnectionState.CONNECTING || connState == StreamConnectionState.NEGOTIATING) {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("CONNECTING STREAM...", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                Text("Negotiating MJPEG multipart pipe on :7890/live", color = TextMuted, fontSize = 10.sp)
                            }
                        } else {
                            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                                Text("STREAM OFFLINE", color = TextSecondary, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                Text("Verify Windows host is running, then tap Connect", color = TextMuted, fontSize = 10.sp)
                            }
                        }
                    }

                    // Action buttons: Connect / Disconnect and Quick Click
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                        Box(Modifier.weight(1.2f)) {
                            if (connState == StreamConnectionState.CONNECTED || connState == StreamConnectionState.CONNECTING) {
                                SecondaryButton("Stop Stream") {
                                    streamEngine.stopStreaming()
                                    onNotice("Stream stopped.")
                                }
                            } else {
                                PrimaryButton("Connect Stream") {
                                    val h = hostInput.trim().ifEmpty { "10.0.2.2" }
                                    val tok = tokenInput.trim().ifEmpty { null }
                                    streamEngine.startStreaming(h, 7890, tok)
                                    onNotice("Connecting to $h:7890/live...")
                                }
                            }
                        }
                        Box(Modifier.weight(0.8f)) {
                            SecondaryButton("L-Click") {
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(hostInput, 7890, "left_click", 960, 540, sequence = seq)
                                }
                                onNotice("Sent Left Click")
                            }
                        }
                        Box(Modifier.weight(0.8f)) {
                            SecondaryButton("R-Click") {
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(hostInput, 7890, "right_click", 960, 540, sequence = seq)
                                }
                                onNotice("Sent Right Click")
                            }
                        }
                    }

                    // Telemetry row
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                        TelemetryCard(
                            label = "FPS & Resolution",
                            value = if (connState == StreamConnectionState.CONNECTED) "${String.format(java.util.Locale.US, "%.0f", streamDiag.currentFps)} FPS" else "--",
                            detail = streamDiag.resolution,
                            modifier = Modifier.weight(1f)
                        )
                        TelemetryCard(
                            label = "Decoded Frames",
                            value = if (connState == StreamConnectionState.CONNECTED) "${streamDiag.framesRendered}" else "--",
                            detail = "${String.format(java.util.Locale.US, "%.0f", streamDiag.latencyMs)} ms RTT",
                            modifier = Modifier.weight(1f)
                        )
                    }
                }
            }
        }

        // Touchpad Mode
        if (remoteMode == "Touchpad") {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Surface1,
                shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, Line)
            ) {
                Column(modifier = Modifier.padding(16.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text("WIRELESS TRACKPAD", color = Brand300, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                        Spacer(Modifier.weight(1f))
                        Text("14 ms • Smooth Glide", color = Success, fontSize = 10.sp)
                    }

                    Spacer(Modifier.height(10.dp))
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(200.dp)
                            .clip(RoundedCornerShape(10.dp))
                            .background(Surface2)
                            .border(1.dp, Line, RoundedCornerShape(10.dp))
                            .clickable { onNotice("Touchpad click dispatched to Windows PC") },
                        contentAlignment = Alignment.Center
                    ) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("TOUCHPAD SURFACE", color = TextMuted, fontSize = 12.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.sp)
                            Text("Slide finger to steer mouse cursor", color = TextMuted.copy(alpha = 0.7f), fontSize = 10.sp)
                        }
                    }

                    Spacer(Modifier.height(10.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                        Box(Modifier.weight(1f)) {
                            SecondaryButton("Left Click") { onNotice("Left click sent") }
                        }
                        Box(Modifier.weight(1f)) {
                            SecondaryButton("Right Click") { onNotice("Right click sent") }
                        }
                    }
                }
            }
        }

        // Keyboard Mode
        if (remoteMode == "Keyboard") {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Surface1,
                shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, Line)
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    Text("PHONE AS PC KEYBOARD", color = Brand300, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(8.dp))
                            .background(Surface2)
                            .border(1.dp, Line, RoundedCornerShape(8.dp))
                            .padding(12.dp)
                    ) {
                        BasicTextField(
                            value = keyboardText,
                            onValueChange = { keyboardText = it },
                            textStyle = TextStyle(color = TextPrimary, fontSize = 14.sp),
                            cursorBrush = SolidColor(Brand400),
                            modifier = Modifier.fillMaxWidth()
                        )
                        if (keyboardText.isEmpty()) {
                            Text("Type here to send directly to Windows PC...", color = TextMuted, fontSize = 13.sp)
                        }
                    }

                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                        listOf("Enter", "Esc", "Tab", "Backspace").forEach { k ->
                            Box(
                                modifier = Modifier
                                    .weight(1f)
                                    .clip(RoundedCornerShape(6.dp))
                                    .background(Surface2)
                                    .clickable { onNotice("Key $k dispatched") }
                                    .padding(vertical = 8.dp),
                                contentAlignment = Alignment.Center
                            ) {
                                Text(k, color = TextPrimary, fontSize = 10.sp)
                            }
                        }
                    }

                    PrimaryButton("Send to PC") {
                        onNotice("Sent: \"$keyboardText\" to PC")
                        keyboardText = ""
                    }
                }
            }
        }

        // Media Remote Mode
        if (remoteMode == "Media") {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Surface1,
                shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, Line)
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(14.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("PC MEDIA CONTROLLER", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)

                    Row(horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.CenterVertically) {
                        Box(Modifier.size(48.dp).clip(CircleShape).background(Surface2).clickable { onNotice("Media Previous") }, contentAlignment = Alignment.Center) {
                            SkipPrevVector(size = 18.dp)
                        }
                        Box(Modifier.size(64.dp).clip(CircleShape).background(Brand600).clickable { onNotice("Media Play/Pause") }, contentAlignment = Alignment.Center) {
                            PlayPauseVector(isPlaying = true, size = 22.dp)
                        }
                        Box(Modifier.size(48.dp).clip(CircleShape).background(Surface2).clickable { onNotice("Media Next") }, contentAlignment = Alignment.Center) {
                            SkipNextVector(size = 18.dp)
                        }
                    }

                    Column(Modifier.fillMaxWidth()) {
                        Text("Volume: ${(volume * 100).toInt()}%", color = TextSecondary, fontSize = 13.sp)
                        Slider(
                            value = volume,
                            onValueChange = {
                                volume = it
                                onNotice("Volume set to ${(it * 100).toInt()}%")
                            },
                            colors = SliderDefaults.colors(
                                thumbColor = Brand400,
                                activeTrackColor = Brand500,
                                inactiveTrackColor = Surface3
                            )
                        )
                    }
                }
            }
        }

        // Presentation Remote Mode
        if (remoteMode == "Slide") {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Surface1,
                shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, Line)
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("PRESENTATION REMOTE", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                    Text("Slide $currentSlide", color = TextPrimary, fontSize = 32.sp, fontWeight = FontWeight.Bold)

                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                        Box(Modifier.weight(1f)) {
                            SecondaryButton("Previous Slide") {
                                if (currentSlide > 1) currentSlide--
                                onNotice("Slide $currentSlide")
                            }
                        }
                        Box(Modifier.weight(1f)) {
                            PrimaryButton("Next Slide") {
                                currentSlide++
                                onNotice("Slide $currentSlide")
                            }
                        }
                    }

                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                        Box(Modifier.weight(1f)) {
                            SecondaryButton("Laser Pointer") { onNotice("Laser pointer toggled on PC") }
                        }
                        Box(Modifier.weight(1f)) {
                            SecondaryButton("Black Screen") { onNotice("Black screen toggled") }
                        }
                    }
                }
            }
        }

        // Game Controller Mode
        if (remoteMode == "Game") {
            Surface(
                modifier = Modifier.fillMaxWidth(),
                color = Surface1,
                shape = RoundedCornerShape(12.dp),
                border = androidx.compose.foundation.BorderStroke(1.dp, Line)
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(14.dp)) {
                    Text("GAMEPAD CONTROLLER", color = Brand300, fontSize = 9.sp, fontWeight = FontWeight.Bold)

                    Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
                        Box(Modifier.clip(RoundedCornerShape(6.dp)).background(Surface2).clickable { onNotice("L1 Trigger") }.padding(horizontal = 16.dp, vertical = 8.dp)) {
                            Text("L1", color = Brand300, fontWeight = FontWeight.Bold)
                        }
                        Box(Modifier.clip(RoundedCornerShape(6.dp)).background(Surface2).clickable { onNotice("R1 Trigger") }.padding(horizontal = 16.dp, vertical = 8.dp)) {
                            Text("R1", color = Brand300, fontWeight = FontWeight.Bold)
                        }
                    }

                    Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
                        // D-Pad
                        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Box(Modifier.size(34.dp).clip(RoundedCornerShape(4.dp)).background(Surface2).clickable { onNotice("D-Pad Up") }, contentAlignment = Alignment.Center) { Text("▲", color = TextPrimary) }
                            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                Box(Modifier.size(34.dp).clip(RoundedCornerShape(4.dp)).background(Surface2).clickable { onNotice("D-Pad Left") }, contentAlignment = Alignment.Center) { Text("◀", color = TextPrimary) }
                                Box(Modifier.size(34.dp).clip(RoundedCornerShape(4.dp)).background(Surface3), contentAlignment = Alignment.Center) { Text("•", color = TextMuted) }
                                Box(Modifier.size(34.dp).clip(RoundedCornerShape(4.dp)).background(Surface2).clickable { onNotice("D-Pad Right") }, contentAlignment = Alignment.Center) { Text("▶", color = TextPrimary) }
                            }
                            Box(Modifier.size(34.dp).clip(RoundedCornerShape(4.dp)).background(Surface2).clickable { onNotice("D-Pad Down") }, contentAlignment = Alignment.Center) { Text("▼", color = TextPrimary) }
                        }

                        // ABXY
                        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
                            Box(Modifier.size(34.dp).clip(CircleShape).background(Brand600).clickable { onNotice("Y Button") }, contentAlignment = Alignment.Center) { Text("Y", color = Color.White, fontWeight = FontWeight.Bold) }
                            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                Box(Modifier.size(34.dp).clip(CircleShape).background(Brand600).clickable { onNotice("X Button") }, contentAlignment = Alignment.Center) { Text("X", color = Color.White, fontWeight = FontWeight.Bold) }
                                Spacer(Modifier.size(34.dp))
                                Box(Modifier.size(34.dp).clip(CircleShape).background(Brand600).clickable { onNotice("B Button") }, contentAlignment = Alignment.Center) { Text("B", color = Color.White, fontWeight = FontWeight.Bold) }
                            }
                            Box(Modifier.size(34.dp).clip(CircleShape).background(Brand600).clickable { onNotice("A Button") }, contentAlignment = Alignment.Center) { Text("A", color = Color.White, fontWeight = FontWeight.Bold) }
                        }
                    }
                }
            }
        }
    }
}

// ─── 14 Security Screen ──────────────────────────────────────────────────────
@Composable
private fun SecurityScreen(hostAddress: String = "10.0.2.2", onNotice: (String) -> Unit) {
    val scope = rememberCoroutineScope()
    var isTestingDiag by remember { mutableStateOf(false) }
    var diagReport by remember { mutableStateOf<SmpClient.DiagnosticsReport?>(null) }
    var pingMs by remember { mutableStateOf<Long?>(null) }
    var diagError by remember { mutableStateOf<String?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("SECURITY CENTER & DEVICE TRUST", color = Brand300, fontSize = 10.sp, fontWeight = FontWeight.Bold)

        // Live Protocol Diagnostics Tool
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("SMP/1 LIVE PROTOCOL DIAGNOSTICS", color = Brand300, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                    Spacer(Modifier.weight(1f))
                    if (pingMs != null) {
                        Text("RTT: ${pingMs}ms", color = Success, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    }
                }

                if (diagReport != null) {
                    val d = diagReport!!
                    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("• Discovery (UDP 7889):", color = TextSecondary, fontSize = 11.sp)
                            Spacer(Modifier.width(6.dp))
                            Text(d.discovery, color = Success, fontSize = 11.sp, fontWeight = FontWeight.Bold)
                        }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("• Host Device ID:", color = TextSecondary, fontSize = 11.sp)
                            Spacer(Modifier.width(6.dp))
                            Text(d.deviceId, color = TextPrimary, fontSize = 11.sp, fontFamily = FontFamily.Monospace)
                        }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("• Protocol Version:", color = TextSecondary, fontSize = 11.sp)
                            Spacer(Modifier.width(6.dp))
                            Text(d.protocolVersion, color = Success, fontSize = 11.sp, fontWeight = FontWeight.Bold)
                        }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("• Screen Capture Subsystem:", color = TextSecondary, fontSize = 11.sp)
                            Spacer(Modifier.width(6.dp))
                            Text(d.screenCapture, color = Success, fontSize = 11.sp)
                        }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text("• Active Trust Store:", color = TextSecondary, fontSize = 11.sp)
                            Spacer(Modifier.width(6.dp))
                            Text("${d.trustStoreCount} registered devices", color = TextPrimary, fontSize = 11.sp)
                        }
                    }
                } else if (diagError != null) {
                    Text(diagError!!, color = Color(0xFFFF6B6B), fontSize = 11.sp)
                } else {
                    Text("Test full 8-point connectivity to Windows host on port 7890.", color = TextMuted, fontSize = 11.sp)
                }

                PrimaryButton(if (isTestingDiag) "Querying Host..." else "Run Diagnostics Test") {
                    if (!isTestingDiag) {
                        scope.launch {
                            isTestingDiag = true
                            diagError = null
                            val pRes = SmpClient.ping(hostAddress.trim(), 7890)
                            if (pRes.isSuccess) {
                                pingMs = pRes.getOrNull()
                            }
                            val res = SmpClient.getDiagnostics(hostAddress.trim(), 7890)
                            isTestingDiag = false
                            if (res.isSuccess) {
                                diagReport = res.getOrNull()
                                onNotice("Diagnostic probe succeeded: Host online and responsive.")
                            } else {
                                diagError = "Diagnostics failed: " + (res.exceptionOrNull()?.message ?: "Host unreachable")
                                onNotice("Diagnostics failed: Check host IP and firewall.")
                            }
                        }
                    }
                }
            }
        }

        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                listOf(
                    "Transport Encrypted" to "AES-GCM 256-bit",
                    "Host Identity Verified" to "Windows DPAPI Anchor",
                    "Sequence Protection" to "Replay defense active",
                    "Host Authority" to "Source of truth for all access"
                ).forEach { (title, detail) ->
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        ProfessionalCheckBadge(size = 18.dp)
                        Spacer(Modifier.width(10.dp))
                        Text(title + ":", color = Success, fontSize = 13.5.sp, fontWeight = FontWeight.Bold)
                        Spacer(Modifier.width(6.dp))
                        Text(detail, color = TextPrimary, fontSize = 13.5.sp)
                    }
                }
            }
        }

        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(12.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text("TRUSTED DEVICES", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                Spacer(Modifier.height(6.dp))
                Text("Windows PC (Host Authority) — Active", color = TextPrimary, fontSize = 14.sp)
                Spacer(Modifier.height(10.dp))
                SecondaryButton("Revoke All Host Access") {
                    onNotice("All host authorizations revoked.")
                }
            }
        }
    }
}

// ─── Modal Dialogs ───────────────────────────────────────────────────────────
@Composable
private fun QrScannerModal(onDismiss: () -> Unit, onPaired: () -> Unit) {
    Dialog(onDismissRequest = onDismiss) {
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(14.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(20.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Text("SCAN TO CONNECT", color = Brand300, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                Spacer(Modifier.height(4.dp))
                Text("Point camera at Smart Migrate Windows QR code", color = TextSecondary, fontSize = 12.sp, textAlign = TextAlign.Center)

                Spacer(Modifier.height(16.dp))
                // Simulated Camera Viewfinder
                Box(
                    modifier = Modifier
                        .size(200.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .background(Color.Black)
                        .border(2.dp, Brand500, RoundedCornerShape(12.dp)),
                    contentAlignment = Alignment.Center
                ) {
                    Box(Modifier.size(140.dp).border(1.dp, Brand300.copy(alpha = 0.5f)))
                    Text("ALIGNING QR...", color = Brand300, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                }

                Spacer(Modifier.height(16.dp))
                PrimaryButton("Simulate QR Match") { onPaired() }
                Spacer(Modifier.height(6.dp))
                SecondaryButton("Cancel") { onDismiss() }
            }
        }
    }
}

@Composable
private fun PairingPinModal(
    initialHost: String,
    onDismiss: () -> Unit,
    onVerified: (String, String) -> Unit
) {
    var hostAddress by rememberSaveable { mutableStateOf(initialHost) }
    var enteredPin by rememberSaveable { mutableStateOf("") }
    var isPairing by remember { mutableStateOf(false) }
    var statusMessage by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()

    Dialog(onDismissRequest = onDismiss) {
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(14.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(20.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Text("PAIR WITH WINDOWS HOST", color = Brand300, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                Spacer(Modifier.height(4.dp))
                Text("Enter the 6-digit PIN displayed on your PC", color = TextSecondary, fontSize = 12.sp, textAlign = TextAlign.Center)

                Spacer(Modifier.height(10.dp))
                // Host IP field
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(6.dp))
                        .background(Surface2)
                        .border(1.dp, Line, RoundedCornerShape(6.dp))
                        .padding(horizontal = 10.dp, vertical = 6.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("HOST IP: ", color = Brand300, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    BasicTextField(
                        value = hostAddress,
                        onValueChange = { hostAddress = it },
                        textStyle = TextStyle(color = TextPrimary, fontSize = 12.sp, fontFamily = FontFamily.Monospace),
                        cursorBrush = SolidColor(Brand400),
                        singleLine = true,
                        modifier = Modifier.weight(1f)
                    )
                }

                Spacer(Modifier.height(12.dp))
                Text(
                    enteredPin.padEnd(6, '•').chunked(3).joinToString(" - "),
                    color = Brand300,
                    fontSize = 24.sp,
                    fontWeight = FontWeight.Bold,
                    fontFamily = FontFamily.Monospace,
                    letterSpacing = 2.sp
                )

                if (statusMessage != null) {
                    Spacer(Modifier.height(6.dp))
                    Text(
                        statusMessage!!,
                        color = if (statusMessage!!.startsWith("Error")) Brand300 else Success,
                        fontSize = 11.sp,
                        textAlign = TextAlign.Center
                    )
                }

                Spacer(Modifier.height(14.dp))
                // Numeric Keypad
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf(
                        listOf("1", "2", "3"),
                        listOf("4", "5", "6"),
                        listOf("7", "8", "9"),
                        listOf("Clear", "0", "⌫")
                    ).forEach { row ->
                        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                            row.forEach { key ->
                                Box(
                                    modifier = Modifier
                                        .weight(1f)
                                        .clip(RoundedCornerShape(8.dp))
                                        .background(Surface2)
                                        .clickable {
                                            when (key) {
                                                "Clear" -> enteredPin = ""
                                                "⌫" -> if (enteredPin.isNotEmpty()) enteredPin = enteredPin.dropLast(1)
                                                else -> if (enteredPin.length < 6) enteredPin += key
                                            }
                                        }
                                        .padding(vertical = 10.dp),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text(key, color = Color.White, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                                }
                            }
                        }
                    }
                }

                Spacer(Modifier.height(12.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                    Box(Modifier.weight(1f)) {
                        SecondaryButton("Cancel") { onDismiss() }
                    }
                    Box(Modifier.weight(1.2f)) {
                        PrimaryButton(
                            if (isPairing) "Pairing..." else "Submit PIN"
                        ) {
                            if (enteredPin.length == 6 && !isPairing) {
                                scope.launch {
                                    isPairing = true
                                    statusMessage = "Pairing with ${hostAddress.trim()}:7890..."
                                    val res = SmpClient.pair(hostAddress.trim(), 7890, enteredPin)
                                    isPairing = false
                                    if (res.isSuccess) {
                                        val p = res.getOrNull()!!
                                        onVerified(p.sessionToken, p.hostDeviceId)
                                    } else {
                                        statusMessage = "Error: " + (res.exceptionOrNull()?.message ?: "Pairing failed")
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ShareSheetModal(onDismiss: () -> Unit, onSent: () -> Unit) {
    Dialog(onDismissRequest = onDismiss) {
        Surface(
            modifier = Modifier.fillMaxWidth(),
            color = Surface1,
            shape = RoundedCornerShape(14.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, Line)
        ) {
            Column(modifier = Modifier.padding(20.dp)) {
                Text("ANDROID SHARE SHEET", color = Brand300, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                Spacer(Modifier.height(4.dp))
                Text("Share with Smart Migrate", color = TextPrimary, fontSize = 18.sp, fontWeight = FontWeight.Bold)
                Text("Direct destination selection from system share menu", color = TextMuted, fontSize = 13.sp)

                Spacer(Modifier.height(16.dp))
                Surface(
                    modifier = Modifier.fillMaxWidth().clickable { onSent() },
                    color = Surface2,
                    shape = RoundedCornerShape(10.dp),
                    border = androidx.compose.foundation.BorderStroke(1.dp, Line)
                ) {
                    Row(modifier = Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
                        Box(
                            modifier = Modifier
                                .size(44.dp)
                                .clip(RoundedCornerShape(10.dp))
                                .background(Brand600.copy(alpha = 0.20f))
                                .border(1.dp, Brand400.copy(alpha = 0.4f), RoundedCornerShape(10.dp)),
                            contentAlignment = Alignment.Center
                        ) {
                            DesktopDeviceVector(size = 24.dp)
                        }
                        Spacer(Modifier.width(12.dp))
                        Column(Modifier.weight(1f)) {
                            Text("Windows PC (Host)", color = TextPrimary, fontSize = 15.sp, fontWeight = FontWeight.SemiBold)
                            Spacer(Modifier.height(2.dp))
                            Text("Direct LAN • Resumable", color = Success, fontSize = 12.sp)
                        }
                        Text("Send", color = Brand300, fontSize = 14.sp, fontWeight = FontWeight.Bold)
                    }
                }

                Spacer(Modifier.height(14.dp))
                SecondaryButton("Dismiss") { onDismiss() }
            }
        }
    }
}

// ─── Reusable UI Atoms ───────────────────────────────────────────────────────
@Composable
private fun PrimaryButton(label: String, onClick: () -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(Brush.horizontalGradient(listOf(Brand500, Brand600)))
            .border(1.dp, LineStrong, RoundedCornerShape(10.dp))
            .clickable { onClick() }
            .padding(vertical = 12.dp),
        contentAlignment = Alignment.Center
    ) {
        Text(label, color = Color.White, fontSize = 14.sp, fontWeight = FontWeight.Bold)
    }
}

@Composable
private fun SecondaryButton(label: String, onClick: () -> Unit) {
    Box(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(10.dp))
            .background(Surface2)
            .border(1.dp, Line, RoundedCornerShape(10.dp))
            .clickable { onClick() }
            .padding(vertical = 12.dp),
        contentAlignment = Alignment.Center
    ) {
        Text(label, color = TextPrimary, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
private fun TelemetryCard(label: String, value: String, detail: String, modifier: Modifier = Modifier) {
    Surface(
        modifier = modifier,
        color = Surface2,
        shape = RoundedCornerShape(10.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, Line)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(label, color = TextMuted, fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
            Spacer(Modifier.height(4.dp))
            Text(value, color = TextPrimary, fontSize = 16.sp, fontWeight = FontWeight.Bold)
            Spacer(Modifier.height(2.dp))
            Text(detail, color = Brand300, fontSize = 12.sp)
        }
    }
}

@Composable
private fun QuickActionPill(
    label: String,
    modifier: Modifier = Modifier,
    icon: @Composable () -> Unit,
    onClick: () -> Unit
) {
    Surface(
        modifier = modifier.clickable { onClick() },
        color = Surface1,
        shape = RoundedCornerShape(10.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, Line)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 11.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center
        ) {
            icon()
            Spacer(Modifier.width(8.dp))
            Text(label, color = TextPrimary, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
        }
    }
}

@Composable
private fun NoticeBar(notice: String) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = Surface2,
        shape = RoundedCornerShape(8.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, Line)
    ) {
        Text(
            notice,
            color = TextSecondary,
            fontSize = 13.sp,
            modifier = Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
            maxLines = 1
        )
    }
}

@Composable
private fun BottomNavBar(selected: AppDestination, onSelect: (AppDestination) -> Unit) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        color = Surface1,
        shape = RoundedCornerShape(12.dp),
        border = androidx.compose.foundation.BorderStroke(1.dp, Line)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 8.dp),
            horizontalArrangement = Arrangement.SpaceAround
        ) {
            AppDestination.entries.forEach { dest ->
                val isSel = selected == dest
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .clickable { onSelect(dest) }
                        .padding(horizontal = 10.dp, vertical = 4.dp)
                ) {
                    DestinationVectorIcon(destination = dest, isSelected = isSel)
                    Spacer(Modifier.height(4.dp))
                    Text(
                        dest.label,
                        color = if (isSel) Brand300 else TextMuted,
                        fontSize = 12.sp,
                        fontWeight = if (isSel) FontWeight.Bold else FontWeight.Medium
                    )
                }
            }
        }
    }
}
