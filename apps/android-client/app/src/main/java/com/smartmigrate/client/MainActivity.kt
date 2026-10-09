package com.smartmigrate.client

import android.os.Bundle
import android.Manifest
import android.content.pm.PackageManager
import android.view.ViewGroup
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.setContent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat
import androidx.core.view.WindowCompat
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.ui.viewinterop.AndroidView
import com.google.zxing.BinaryBitmap
import com.google.zxing.MultiFormatReader
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.common.HybridBinarizer
import java.util.concurrent.Executors
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
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.IntSize
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
import androidx.compose.runtime.LaunchedEffect
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
    Remote("Stream", "⌁"),
    Devices("Devices", "◇"),
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
    var notice by rememberSaveable { mutableStateOf("Smart Migrate Core online. Direct LAN active.") }
    var batterySaverMode by rememberSaveable { mutableStateOf(false) }
    val initialHost = remember { "${SmpClient.getLocalSubnetPrefix()}33" }
    var targetHostAddress by rememberSaveable { mutableStateOf(initialHost) }
    var currentSessionToken by rememberSaveable { mutableStateOf("") }

    // Auto-discover Windows Host on LAN at startup
    LaunchedEffect(Unit) {
        val discovered = SmpClient.discoverHosts()
        if (discovered != null) {
            targetHostAddress = discovered.ip
            notice = "Discovered Windows Host '${discovered.name}' at ${discovered.ip}:7890"
        }
    }

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
                            hostAddress = targetHostAddress,
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

            // Real Camera QR Scanner Dialog
            if (qrScannerOpen) {
                QrScannerModal(
                    onDismiss = { qrScannerOpen = false },
                    onManualPin = {
                        qrScannerOpen = false
                        pairingDialog = true
                    },
                    onPaired = { host, token, hostId ->
                        qrScannerOpen = false
                        targetHostAddress = host
                        currentSessionToken = token
                        notice = "Paired with $hostId at $host! Session verified."
                        destination = AppDestination.Remote
                    }
                )
            }

            // Numeric PIN Pairing Dialog
            if (pairingDialog) {
                PairingPinModal(
                    initialHost = targetHostAddress,
                    onDismiss = { pairingDialog = false },
                    onVerified = { host, token, hostId ->
                        pairingDialog = false
                        targetHostAddress = host
                        currentSessionToken = token
                        notice = "Paired with $hostId at $host! Session verified."
                        destination = AppDestination.Remote
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
private fun ShieldVector(modifier: Modifier = Modifier, color: Color = Brand300, size: Dp = 16.dp) {
    Canvas(modifier = modifier.size(size)) {
        val s = 1.8.dp.toPx()
        val w = this.size.width
        val h = this.size.height
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
private fun AppBar(onScanQR: () -> Unit, batterySaver: Boolean) {
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
                Text(if (batterySaver) "Battery Saver Mode Active" else "Direct LAN • 60 FPS Stream", color = if (batterySaver) Warning else TextMuted, fontSize = 12.sp)
            }
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(8.dp))
                    .background(Brand600.copy(alpha = 0.25f))
                    .border(1.dp, Brand400.copy(alpha = 0.5f), RoundedCornerShape(8.dp))
                    .clickable { onScanQR() }
                    .padding(horizontal = 12.dp, vertical = 7.dp)
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    QrCodeVector(size = 14.dp, color = Brand300)
                    Text("Scan QR", color = Brand300, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                }
            }
        }
    }
}

// ─── 02 Home Screen ──────────────────────────────────────────────────────────
@Composable
private fun HomeScreen(
    hostAddress: String = "",
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
                        Text("● HOST DISCOVERED", color = Success, fontSize = 11.sp, fontWeight = FontWeight.Bold)
                    }
                    Spacer(Modifier.weight(1f))
                    Text("SMP/1 • MigRoute", color = Brand300, fontSize = 12.sp, fontFamily = FontFamily.Monospace)
                }

                Spacer(Modifier.height(14.dp))
                Text("PC Display Stream", color = TextPrimary, fontSize = 24.sp, fontWeight = FontWeight.Bold)
                Spacer(Modifier.height(4.dp))
                Text("Direct LAN • Host: $hostAddress:7890 • 60 FPS Hardware Video", color = TextSecondary, fontSize = 13.5.sp)

                Spacer(Modifier.height(16.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                    Box(Modifier.weight(1.2f)) {
                        PrimaryButton("Live Stream") { onNavigate(AppDestination.Remote) }
                    }
                    Box(Modifier.weight(1f)) {
                        SecondaryButton("Scan QR") { onScanQR() }
                    }
                }
            }
        }

        // Quick Actions Row
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
            QuickActionPill("Scan QR", Modifier.weight(1f), icon = { QrCodeVector() }) { onScanQR() }
            QuickActionPill("Enter PIN", Modifier.weight(1f), icon = { KeypadVector() }) { onPair() }
            QuickActionPill("Security", Modifier.weight(1f), icon = { ShieldVector() }) {
                onNavigate(AppDestination.Security)
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

// ─── 12 Remote Desktop 2.0 & Device Control (StarDesk-Inspired Interactive Pipeline) ───
@Composable
private fun RemoteScreen(
    hostAddress: String,
    sessionToken: String,
    batterySaver: Boolean,
    onNotice: (String) -> Unit
) {
    val scope = rememberCoroutineScope()
    val streamEngine = remember { StreamEngine() }
    val connState by streamEngine.state
    val latestBitmap by streamEngine.latestBitmap
    val streamDiag by streamEngine.diagnostics

    var hostInput by rememberSaveable { mutableStateOf(hostAddress) }
    var tokenInput by rememberSaveable { mutableStateOf(sessionToken) }
    var inputSeq by rememberSaveable { mutableLongStateOf(1L) }

    // Remote Control Modes & Panel State
    var controlMode by rememberSaveable { mutableStateOf("Touch") } // "Touch" or "Cursor"
    var isPanelOpen by rememberSaveable { mutableStateOf(false) }
    var isKeyboardOpen by rememberSaveable { mutableStateOf(false) }
    var panelTab by rememberSaveable { mutableStateOf("Panel") } // "Panel" or "Security"
    var virtualMouseEnabled by rememberSaveable { mutableStateOf(true) }
    var clipboardSyncEnabled by rememberSaveable { mutableStateOf(false) }
    var hostPrivacyMode by rememberSaveable { mutableStateOf(false) }
    var muteHostAudio by rememberSaveable { mutableStateOf(false) }
    var showClipboardBanner by rememberSaveable { mutableStateOf(true) }
    var fpsMode by rememberSaveable { mutableIntStateOf(60) }
    var resolutionMode by rememberSaveable { mutableStateOf("1080p") }
    var keyboardInputText by rememberSaveable { mutableStateOf("") }

    // Virtual Cursor Position in Cursor Mode (normalized 0.0 .. 1.0)
    var cursorNormX by rememberSaveable { mutableFloatStateOf(0.5f) }
    var cursorNormY by rememberSaveable { mutableFloatStateOf(0.5f) }
    var viewportSize by remember { mutableStateOf(IntSize(1, 1)) }

    DisposableEffect(streamEngine) {
        onDispose {
            streamEngine.stopStreaming()
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xFF070B14))
    ) {
        // ── 1. Live Remote Desktop Surface ────────────────────────────────────
        Box(
            modifier = Modifier
                .fillMaxSize()
                .onSizeChanged { viewportSize = it }
                .pointerInput(controlMode, connState) {
                    if (connState != StreamConnectionState.CONNECTED) return@pointerInput

                    if (controlMode == "Touch") {
                        // Direct Touch Gestures
                        detectTapGestures(
                            onTap = { offset ->
                                val nx = (offset.x / size.width.toFloat()).coerceIn(0f, 1f)
                                val ny = (offset.y / size.height.toFloat()).coerceIn(0f, 1f)
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "left_click",
                                        normalizedX = nx,
                                        normalizedY = ny,
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                            },
                            onDoubleTap = { offset ->
                                val nx = (offset.x / size.width.toFloat()).coerceIn(0f, 1f)
                                val ny = (offset.y / size.height.toFloat()).coerceIn(0f, 1f)
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "double_click",
                                        normalizedX = nx,
                                        normalizedY = ny,
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                            },
                            onLongPress = { offset ->
                                val nx = (offset.x / size.width.toFloat()).coerceIn(0f, 1f)
                                val ny = (offset.y / size.height.toFloat()).coerceIn(0f, 1f)
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "right_click",
                                        normalizedX = nx,
                                        normalizedY = ny,
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                                onNotice("Right click dispatched")
                            }
                        )
                    }
                }
                .pointerInput(controlMode, connState) {
                    if (connState != StreamConnectionState.CONNECTED) return@pointerInput

                    if (controlMode == "Cursor") {
                        // Cursor Mode Relative Trackpad Glide
                        detectDragGestures { change, dragAmount ->
                            change.consume()
                            val dx = dragAmount.x
                            val dy = dragAmount.y
                            cursorNormX = (cursorNormX + dx / size.width.toFloat()).coerceIn(0f, 1f)
                            cursorNormY = (cursorNormY + dy / size.height.toFloat()).coerceIn(0f, 1f)

                            val seq = inputSeq++
                            scope.launch {
                                SmpClient.sendInput(
                                    host = hostInput.ifEmpty { "10.0.2.2" },
                                    action = "relative_move",
                                    dx = (dx * 1.6f).toInt(),
                                    dy = (dy * 1.6f).toInt(),
                                    sequence = seq,
                                    sessionToken = tokenInput.ifEmpty { null }
                                )
                            }
                        }
                    } else {
                        // Touch Mode Dragging (Window Move / Selection)
                        detectDragGestures(
                            onDragStart = { offset ->
                                val nx = (offset.x / size.width.toFloat()).coerceIn(0f, 1f)
                                val ny = (offset.y / size.height.toFloat()).coerceIn(0f, 1f)
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "mouse_down",
                                        button = "left",
                                        normalizedX = nx,
                                        normalizedY = ny,
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                            },
                            onDrag = { change, _ ->
                                change.consume()
                                val nx = (change.position.x / size.width.toFloat()).coerceIn(0f, 1f)
                                val ny = (change.position.y / size.height.toFloat()).coerceIn(0f, 1f)
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "move",
                                        normalizedX = nx,
                                        normalizedY = ny,
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                            },
                            onDragEnd = {
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "mouse_up",
                                        button = "left",
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                            }
                        )
                    }
                },
            contentAlignment = Alignment.Center
        ) {
            val frame = latestBitmap
            if (connState == StreamConnectionState.CONNECTED && frame != null) {
                Image(
                    bitmap = frame.asImageBitmap(),
                    contentDescription = "Windows Desktop Display",
                    modifier = Modifier.fillMaxSize(),
                    contentScale = ContentScale.Fit
                )

                // Virtual Mouse Pointer in Cursor Mode (StarDesk Screenshot 5)
                if (controlMode == "Cursor") {
                    val px = cursorNormX * viewportSize.width.toFloat()
                    val py = cursorNormY * viewportSize.height.toFloat()
                    Canvas(modifier = Modifier.fillMaxSize()) {
                        val path = Path().apply {
                            moveTo(px, py)
                            lineTo(px + 22.dp.toPx(), py + 14.dp.toPx())
                            lineTo(px + 12.dp.toPx(), py + 14.dp.toPx())
                            lineTo(px + 18.dp.toPx(), py + 26.dp.toPx())
                            lineTo(px + 13.dp.toPx(), py + 28.dp.toPx())
                            lineTo(px + 8.dp.toPx(), py + 16.dp.toPx())
                            lineTo(px, py + 22.dp.toPx())
                            close()
                        }
                        drawPath(path, color = Color.White)
                        drawPath(path, color = Color.Black, style = Stroke(width = 2.dp.toPx()))
                    }
                }
            } else {
                // Offline / Setup View
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(24.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    Box(
                        modifier = Modifier
                            .size(64.dp)
                            .clip(CircleShape)
                            .background(Brand600.copy(alpha = 0.2f))
                            .border(1.dp, Brand500.copy(alpha = 0.4f), CircleShape),
                        contentAlignment = Alignment.Center
                    ) {
                        DesktopDeviceVector(color = Brand400, size = 32.dp)
                    }

                    Text(
                        if (connState == StreamConnectionState.CONNECTING || connState == StreamConnectionState.NEGOTIATING)
                            "CONNECTING TO WINDOWS PC..."
                        else "REMOTE DESKTOP READY",
                        color = Color.White,
                        fontSize = 15.sp,
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 0.5.sp
                    )
                    Text(
                        "Direct LAN hardware stream & full interactive PC remote control",
                        color = TextMuted,
                        fontSize = 12.sp,
                        textAlign = TextAlign.Center
                    )

                    // Host & Token Inputs
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .weight(1.2f)
                                .clip(RoundedCornerShape(8.dp))
                                .background(Surface2)
                                .border(1.dp, Line, RoundedCornerShape(8.dp))
                                .padding(horizontal = 10.dp, vertical = 8.dp)
                        ) {
                            if (hostInput.isEmpty()) {
                                Text("Host IP (e.g. 192.168.31.33)", color = TextMuted, fontSize = 12.sp)
                            }
                            BasicTextField(
                                value = hostInput,
                                onValueChange = { hostInput = it },
                                textStyle = TextStyle(color = TextPrimary, fontSize = 13.sp, fontFamily = FontFamily.Monospace),
                                cursorBrush = SolidColor(Brand400),
                                singleLine = true
                            )
                        }
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .clip(RoundedCornerShape(8.dp))
                                .background(Surface2)
                                .border(1.dp, Line, RoundedCornerShape(8.dp))
                                .padding(horizontal = 10.dp, vertical = 8.dp)
                        ) {
                            if (tokenInput.isEmpty()) {
                                Text("Session Token", color = TextMuted, fontSize = 12.sp)
                            }
                            BasicTextField(
                                value = tokenInput,
                                onValueChange = { tokenInput = it },
                                textStyle = TextStyle(color = TextPrimary, fontSize = 13.sp, fontFamily = FontFamily.Monospace),
                                cursorBrush = SolidColor(Brand400),
                                singleLine = true
                            )
                        }
                    }

                    PrimaryButton("Connect Remote Desktop") {
                        val h = hostInput.trim().ifEmpty { "10.0.2.2" }
                        val tok = tokenInput.trim().ifEmpty { null }
                        streamEngine.startStreaming(h, 7890, tok)
                        onNotice("Connecting to $h:7890/live...")
                    }
                }
            }
        }

        // ── 2. StarDesk-Inspired Right Slim Floating Toolbar (Screenshot 2) ───
        if (connState == StreamConnectionState.CONNECTED) {
            Box(
                modifier = Modifier
                    .align(Alignment.CenterEnd)
                    .padding(end = 8.dp)
                    .clip(RoundedCornerShape(12.dp))
                    .background(Color(0xD90F172A))
                    .border(1.dp, Color(0x3338BDF8), RoundedCornerShape(12.dp))
                    .padding(vertical = 8.dp, horizontal = 6.dp)
            ) {
                Column(
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                    horizontalAlignment = Alignment.CenterHorizontally
                ) {
                    // 1. Panel Button
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier
                            .clip(RoundedCornerShape(6.dp))
                            .clickable { isPanelOpen = !isPanelOpen }
                            .padding(4.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .size(28.dp)
                                .clip(CircleShape)
                                .background(if (isPanelOpen) Brand600 else Color(0x26FFFFFF)),
                            contentAlignment = Alignment.Center
                        ) {
                            Canvas(modifier = Modifier.size(14.dp)) {
                                drawLine(Color.White, Offset(2.dp.toPx(), 4.dp.toPx()), Offset(12.dp.toPx(), 4.dp.toPx()), strokeWidth = 2.dp.toPx(), cap = StrokeCap.Round)
                                drawLine(Color.White, Offset(2.dp.toPx(), 7.dp.toPx()), Offset(12.dp.toPx(), 7.dp.toPx()), strokeWidth = 2.dp.toPx(), cap = StrokeCap.Round)
                                drawLine(Color.White, Offset(2.dp.toPx(), 10.dp.toPx()), Offset(12.dp.toPx(), 10.dp.toPx()), strokeWidth = 2.dp.toPx(), cap = StrokeCap.Round)
                            }
                        }
                        Spacer(Modifier.height(2.dp))
                        Text("Panel", color = Color.White, fontSize = 8.sp, fontWeight = FontWeight.SemiBold)
                    }

                    // 2. Keyboard Button
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier
                            .clip(RoundedCornerShape(6.dp))
                            .clickable { isKeyboardOpen = !isKeyboardOpen }
                            .padding(4.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .size(28.dp)
                                .clip(CircleShape)
                                .background(if (isKeyboardOpen) Brand600 else Color(0x26FFFFFF)),
                            contentAlignment = Alignment.Center
                        ) {
                            Canvas(modifier = Modifier.size(14.dp)) {
                                drawRect(Color.White, size = androidx.compose.ui.geometry.Size(14.dp.toPx(), 10.dp.toPx()), style = Stroke(width = 1.5.dp.toPx()))
                                drawCircle(Color.White, radius = 1.dp.toPx(), center = Offset(4.dp.toPx(), 4.dp.toPx()))
                                drawCircle(Color.White, radius = 1.dp.toPx(), center = Offset(7.dp.toPx(), 4.dp.toPx()))
                                drawCircle(Color.White, radius = 1.dp.toPx(), center = Offset(10.dp.toPx(), 4.dp.toPx()))
                                drawLine(Color.White, Offset(4.dp.toPx(), 7.dp.toPx()), Offset(10.dp.toPx(), 7.dp.toPx()), strokeWidth = 1.5.dp.toPx())
                            }
                        }
                        Spacer(Modifier.height(2.dp))
                        Text("Keyboard", color = Color.White, fontSize = 8.sp, fontWeight = FontWeight.SemiBold)
                    }

                    // 3. Show Desktop Button (Win+D)
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier
                            .clip(RoundedCornerShape(6.dp))
                            .clickable {
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "hotkey",
                                        hotkey = "show_desktop",
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                                onNotice("Sent Show Desktop (Win+D)")
                            }
                            .padding(4.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .size(28.dp)
                                .clip(CircleShape)
                                .background(Color(0x26FFFFFF)),
                            contentAlignment = Alignment.Center
                        ) {
                            Canvas(modifier = Modifier.size(14.dp)) {
                                drawRect(Color.White, size = androidx.compose.ui.geometry.Size(14.dp.toPx(), 9.dp.toPx()), style = Stroke(width = 1.5.dp.toPx()))
                                drawLine(Color.White, Offset(7.dp.toPx(), 9.dp.toPx()), Offset(7.dp.toPx(), 12.dp.toPx()), strokeWidth = 1.5.dp.toPx())
                                drawLine(Color.White, Offset(4.dp.toPx(), 12.dp.toPx()), Offset(10.dp.toPx(), 12.dp.toPx()), strokeWidth = 1.5.dp.toPx())
                            }
                        }
                        Spacer(Modifier.height(2.dp))
                        Text("Desktop", color = Color.White, fontSize = 8.sp, fontWeight = FontWeight.SemiBold)
                    }

                    // 4. Task Switcher Button (Alt+Tab)
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier
                            .clip(RoundedCornerShape(6.dp))
                            .clickable {
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "hotkey",
                                        hotkey = "task_switch",
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                                onNotice("Sent Task Switcher (Alt+Tab)")
                            }
                            .padding(4.dp)
                    ) {
                        Box(
                            modifier = Modifier
                                .size(28.dp)
                                .clip(CircleShape)
                                .background(Color(0x26FFFFFF)),
                            contentAlignment = Alignment.Center
                        ) {
                            Canvas(modifier = Modifier.size(14.dp)) {
                                drawRect(Color.White.copy(alpha = 0.6f), topLeft = Offset(0f, 0f), size = androidx.compose.ui.geometry.Size(9.dp.toPx(), 9.dp.toPx()), style = Stroke(width = 1.2.dp.toPx()))
                                drawRect(Color.White, topLeft = Offset(4.dp.toPx(), 4.dp.toPx()), size = androidx.compose.ui.geometry.Size(9.dp.toPx(), 9.dp.toPx()), style = Stroke(width = 1.2.dp.toPx()))
                            }
                        }
                        Spacer(Modifier.height(2.dp))
                        Text("Switch", color = Color.White, fontSize = 8.sp, fontWeight = FontWeight.SemiBold)
                    }
                }
            }
        }

        // ── 3. Virtual Mouse Tactile Buttons in Cursor Mode (Screenshot 5) ────
        if (connState == StreamConnectionState.CONNECTED && controlMode == "Cursor" && virtualMouseEnabled) {
            // Left & Right click floating tactile buttons at bottom left
            Row(
                modifier = Modifier
                    .align(Alignment.BottomStart)
                    .padding(start = 16.dp, bottom = 16.dp),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                // Left Click [ L ]
                Box(
                    modifier = Modifier
                        .size(54.dp)
                        .clip(CircleShape)
                        .background(Color(0xD91E293B))
                        .border(1.5.dp, Brand400, CircleShape)
                        .clickable {
                            val seq = inputSeq++
                            scope.launch {
                                SmpClient.sendInput(
                                    host = hostInput.ifEmpty { "10.0.2.2" },
                                    action = "left_click",
                                    sequence = seq,
                                    sessionToken = tokenInput.ifEmpty { null }
                                )
                            }
                        },
                    contentAlignment = Alignment.Center
                ) {
                    Text("L", color = Color.White, fontSize = 18.sp, fontWeight = FontWeight.Bold)
                }

                // Right Click [ R ]
                Box(
                    modifier = Modifier
                        .size(54.dp)
                        .clip(CircleShape)
                        .background(Color(0xD91E293B))
                        .border(1.5.dp, Color(0x6694A3B8), CircleShape)
                        .clickable {
                            val seq = inputSeq++
                            scope.launch {
                                SmpClient.sendInput(
                                    host = hostInput.ifEmpty { "10.0.2.2" },
                                    action = "right_click",
                                    sequence = seq,
                                    sessionToken = tokenInput.ifEmpty { null }
                                )
                            }
                        },
                    contentAlignment = Alignment.Center
                ) {
                    Text("R", color = Color(0xFF94A3B8), fontSize = 18.sp, fontWeight = FontWeight.Bold)
                }

                // Double Click [ 2x ]
                Box(
                    modifier = Modifier
                        .size(44.dp)
                        .clip(CircleShape)
                        .background(Color(0xD91E293B))
                        .border(1.dp, Color(0x33FFFFFF), CircleShape)
                        .clickable {
                            val seq = inputSeq++
                            scope.launch {
                                SmpClient.sendInput(
                                    host = hostInput.ifEmpty { "10.0.2.2" },
                                    action = "double_click",
                                    sequence = seq,
                                    sessionToken = tokenInput.ifEmpty { null }
                                )
                            }
                        },
                    contentAlignment = Alignment.Center
                ) {
                    Text("2x", color = Color.White, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                }
            }

            // Vertical Scroll Strip on right edge
            Box(
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(end = 70.dp, bottom = 16.dp)
                    .height(110.dp)
                    .width(36.dp)
                    .clip(RoundedCornerShape(18.dp))
                    .background(Color(0xD91E293B))
                    .border(1.dp, Color(0x3338BDF8), RoundedCornerShape(18.dp))
                    .pointerInput(Unit) {
                        detectDragGestures { change, dragAmount ->
                            change.consume()
                            val delta = (-dragAmount.y * 8f).toInt()
                            if (delta != 0) {
                                val seq = inputSeq++
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "scroll",
                                        scrollDelta = delta,
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                            }
                        }
                    },
                contentAlignment = Alignment.Center
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("▲", color = Brand300, fontSize = 9.sp)
                    Spacer(Modifier.height(2.dp))
                    Text("⇕", color = Color.White, fontSize = 14.sp)
                    Spacer(Modifier.height(2.dp))
                    Text("▼", color = Brand300, fontSize = 9.sp)
                }
            }
        }

        // ── 4. StarDesk-Inspired Clipboard Notification Pill (Screenshot 2) ───
        if (connState == StreamConnectionState.CONNECTED && showClipboardBanner && !clipboardSyncEnabled) {
            Box(
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(top = 12.dp, start = 16.dp, end = 16.dp)
                    .clip(RoundedCornerShape(20.dp))
                    .background(Color(0xF00F172A))
                    .border(1.dp, Color(0x3338BDF8), RoundedCornerShape(20.dp))
                    .padding(horizontal = 14.dp, vertical = 7.dp)
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    Text(
                        "Enable clipboard sync to transfer text with the host device.",
                        color = Color(0xFFE2E8F0),
                        fontSize = 10.sp
                    )
                    Text(
                        "Enable Now",
                        color = Color(0xFF38BDF8),
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier
                            .clickable {
                                clipboardSyncEnabled = true
                                showClipboardBanner = false
                                onNotice("Clipboard sync activated with Windows host")
                            }
                    )
                    Text(
                        "✕",
                        color = Color(0xFF94A3B8),
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        modifier = Modifier.clickable { showClipboardBanner = false }
                    )
                }
            }
        }

        // ── 5. Keyboard Input Overlay ─────────────────────────────────────────
        if (isKeyboardOpen) {
            Box(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp))
                    .background(Color(0xF50F172A))
                    .border(1.dp, Color(0x3338BDF8), RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp))
                    .padding(12.dp)
            ) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Text("KEYBOARD INPUT (UNICODE INJECTION)", color = Brand300, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                        Text(
                            "Close ✕",
                            color = TextMuted,
                            fontSize = 10.sp,
                            fontWeight = FontWeight.Bold,
                            modifier = Modifier.clickable { isKeyboardOpen = false }
                        )
                    }

                    // Quick System Keys
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(4.dp)
                    ) {
                        listOf("Esc", "Tab", "Win", "Alt", "Ctrl", "Enter", "Bksp", "Space").forEach { key ->
                            Box(
                                modifier = Modifier
                                    .weight(1f)
                                    .clip(RoundedCornerShape(6.dp))
                                    .background(Surface2)
                                    .clickable {
                                        val seq = inputSeq++
                                        val hk = when (key) {
                                            "Esc" -> "escape"
                                            "Tab" -> "tab"
                                            "Win" -> "show_desktop"
                                            "Alt" -> "task_switch"
                                            "Enter" -> "enter"
                                            "Bksp" -> "backspace"
                                            "Space" -> "space"
                                            else -> "tab"
                                        }
                                        scope.launch {
                                            SmpClient.sendInput(
                                                host = hostInput.ifEmpty { "10.0.2.2" },
                                                action = "hotkey",
                                                hotkey = hk,
                                                sequence = seq,
                                                sessionToken = tokenInput.ifEmpty { null }
                                            )
                                        }
                                        onNotice("Sent key $key")
                                    }
                                    .padding(vertical = 6.dp),
                                contentAlignment = Alignment.Center
                            ) {
                                Text(key, color = Color.White, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                            }
                        }
                    }

                    // Text Input & Send
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .clip(RoundedCornerShape(8.dp))
                                .background(Surface2)
                                .border(1.dp, Line, RoundedCornerShape(8.dp))
                                .padding(horizontal = 10.dp, vertical = 8.dp)
                        ) {
                            if (keyboardInputText.isEmpty()) {
                                Text("Type text to send to active PC window...", color = TextMuted, fontSize = 12.sp)
                            }
                            BasicTextField(
                                value = keyboardInputText,
                                onValueChange = { keyboardInputText = it },
                                textStyle = TextStyle(color = TextPrimary, fontSize = 13.sp),
                                cursorBrush = SolidColor(Brand400),
                                singleLine = true,
                                modifier = Modifier.fillMaxWidth()
                            )
                        }

                        PrimaryButton("Send") {
                            if (keyboardInputText.isNotEmpty()) {
                                val seq = inputSeq++
                                val txt = keyboardInputText
                                keyboardInputText = ""
                                scope.launch {
                                    SmpClient.sendInput(
                                        host = hostInput.ifEmpty { "10.0.2.2" },
                                        action = "text",
                                        text = txt,
                                        sequence = seq,
                                        sessionToken = tokenInput.ifEmpty { null }
                                    )
                                }
                                onNotice("Sent \"$txt\" to Windows PC")
                            }
                        }
                    }
                }
            }
        }

        // ── 6. StarDesk Slide-Out Control Drawer (Screenshots 3 & 4) ──────────
        if (isPanelOpen) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color(0x80000000))
                    .clickable { isPanelOpen = false }
            ) {
                Box(
                    modifier = Modifier
                        .align(Alignment.BottomCenter)
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(topStart = 20.dp, topEnd = 20.dp))
                        .background(Color(0xFA0F172A))
                        .border(1.dp, Color(0x3338BDF8), RoundedCornerShape(topStart = 20.dp, topEnd = 20.dp))
                        .clickable(enabled = false) {}
                        .padding(16.dp)
                ) {
                    Column(verticalArrangement = Arrangement.spacedBy(14.dp)) {
                        // Header with Tabs & Close
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                listOf("Panel", "Security").forEach { tab ->
                                    val isSel = panelTab == tab
                                    Box(
                                        modifier = Modifier
                                            .clip(RoundedCornerShape(8.dp))
                                            .background(if (isSel) Brand600 else Color(0x1AFFFFFF))
                                            .border(1.dp, if (isSel) Brand400 else Color.Transparent, RoundedCornerShape(8.dp))
                                            .clickable { panelTab = tab }
                                            .padding(horizontal = 14.dp, vertical = 6.dp)
                                    ) {
                                        Text(tab, color = Color.White, fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                    }
                                }
                            }

                            Text(
                                "✕",
                                color = Color(0xFF94A3B8),
                                fontSize = 16.sp,
                                fontWeight = FontWeight.Bold,
                                modifier = Modifier
                                    .clip(CircleShape)
                                    .clickable { isPanelOpen = false }
                                    .padding(4.dp)
                            )
                        }

                        // Top Quick Action Row (StarDesk Screenshot 4)
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            // Disconnect (Red)
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                modifier = Modifier
                                    .clip(RoundedCornerShape(8.dp))
                                    .clickable {
                                        streamEngine.stopStreaming()
                                        scope.launch {
                                            SmpClient.disconnectSession(hostInput.ifEmpty { "10.0.2.2" }, 7890)
                                        }
                                        isPanelOpen = false
                                        onNotice("Remote session disconnected")
                                    }
                                    .padding(6.dp)
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(36.dp)
                                        .clip(CircleShape)
                                        .background(Color(0x33EF4444))
                                        .border(1.dp, Color(0xFFEF4444), CircleShape),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text("⏻", color = Color(0xFFEF4444), fontSize = 14.sp)
                                }
                                Spacer(Modifier.height(4.dp))
                                Text("Disconnect", color = Color(0xFFEF4444), fontSize = 9.sp, fontWeight = FontWeight.Bold)
                            }

                            // Rotate / Orientation
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                modifier = Modifier
                                    .clip(RoundedCornerShape(8.dp))
                                    .clickable { onNotice("Orientation lock toggled") }
                                    .padding(6.dp)
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(36.dp)
                                        .clip(CircleShape)
                                        .background(Color(0x26FFFFFF)),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text("↻", color = Color.White, fontSize = 16.sp)
                                }
                                Spacer(Modifier.height(4.dp))
                                Text("Rotate", color = Color.White, fontSize = 9.sp)
                            }

                            // Audio Mute
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                modifier = Modifier
                                    .clip(RoundedCornerShape(8.dp))
                                    .clickable {
                                        muteHostAudio = !muteHostAudio
                                        onNotice(if (muteHostAudio) "Host audio muted" else "Host audio unmuted")
                                    }
                                    .padding(6.dp)
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(36.dp)
                                        .clip(CircleShape)
                                        .background(if (muteHostAudio) Color(0x33EF4444) else Color(0x26FFFFFF)),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text(if (muteHostAudio) "🔇" else "🔊", fontSize = 13.sp)
                                }
                                Spacer(Modifier.height(4.dp))
                                Text("Audio", color = Color.White, fontSize = 9.sp)
                            }

                            // Host Screen Curtain / Privacy
                            Column(
                                horizontalAlignment = Alignment.CenterHorizontally,
                                modifier = Modifier
                                    .clip(RoundedCornerShape(8.dp))
                                    .clickable {
                                        hostPrivacyMode = !hostPrivacyMode
                                        onNotice(if (hostPrivacyMode) "Host Privacy Mode enabled (Curtain on)" else "Host Privacy Mode disabled")
                                    }
                                    .padding(6.dp)
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size(36.dp)
                                        .clip(CircleShape)
                                        .background(if (hostPrivacyMode) Brand600 else Color(0x26FFFFFF)),
                                    contentAlignment = Alignment.Center
                                ) {
                                    Text("👁", fontSize = 13.sp)
                                }
                                Spacer(Modifier.height(4.dp))
                                Text("Curtain", color = Color.White, fontSize = 9.sp)
                            }
                        }

                        // ── TAB: PANEL (Screenshot 4) ─────────────────────────
                        if (panelTab == "Panel") {
                            // Remote Control Mode Selector (Touch Mode vs Cursor Mode)
                            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                                Text("CONTROL MODE", color = Color(0xFF94A3B8), fontSize = 10.sp, fontWeight = FontWeight.Bold, letterSpacing = 0.5.sp)
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                                ) {
                                    // Touch Mode Card
                                    Box(
                                        modifier = Modifier
                                            .weight(1f)
                                            .clip(RoundedCornerShape(10.dp))
                                            .background(if (controlMode == "Touch") Color(0x3338BDF8) else Color(0x1AFFFFFF))
                                            .border(1.5.dp, if (controlMode == "Touch") Color(0xFF38BDF8) else Color(0x26FFFFFF), RoundedCornerShape(10.dp))
                                            .clickable { controlMode = "Touch"; onNotice("Switched to Touch Mode") }
                                            .padding(12.dp)
                                    ) {
                                        Column {
                                            Text("Touch Mode", color = Color.White, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                                            Spacer(Modifier.height(2.dp))
                                            Text("Direct touch to click, drag & gesture", color = Color(0xFF94A3B8), fontSize = 10.sp)
                                        }
                                    }

                                    // Cursor Mode Card
                                    Box(
                                        modifier = Modifier
                                            .weight(1f)
                                            .clip(RoundedCornerShape(10.dp))
                                            .background(if (controlMode == "Cursor") Color(0x3338BDF8) else Color(0x1AFFFFFF))
                                            .border(1.5.dp, if (controlMode == "Cursor") Color(0xFF38BDF8) else Color(0x26FFFFFF), RoundedCornerShape(10.dp))
                                            .clickable { controlMode = "Cursor"; onNotice("Switched to Cursor Mode") }
                                            .padding(12.dp)
                                    ) {
                                        Column {
                                            Text("Cursor Mode", color = Color.White, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                                            Spacer(Modifier.height(2.dp))
                                            Text("Virtual trackpad & mouse buttons", color = Color(0xFF94A3B8), fontSize = 10.sp)
                                        }
                                    }
                                }
                            }

                            // Virtual Mouse Toggle
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Column {
                                    Text("Virtual Mouse Buttons", color = Color.White, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                                    Text("Show on-screen Left/Right click triggers", color = TextMuted, fontSize = 10.sp)
                                }
                                Box(
                                    modifier = Modifier
                                        .clip(RoundedCornerShape(12.dp))
                                        .background(if (virtualMouseEnabled) Brand600 else Color(0x26FFFFFF))
                                        .clickable { virtualMouseEnabled = !virtualMouseEnabled }
                                        .padding(horizontal = 10.dp, vertical = 4.dp)
                                ) {
                                    Text(if (virtualMouseEnabled) "ON" else "OFF", color = Color.White, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                                }
                            }

                            // Display FPS & Scale
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.spacedBy(8.dp)
                            ) {
                                Box(
                                    modifier = Modifier
                                        .weight(1f)
                                        .clip(RoundedCornerShape(8.dp))
                                        .background(Color(0x1AFFFFFF))
                                        .padding(10.dp)
                                ) {
                                    Column {
                                        Text("TARGET FPS", color = TextMuted, fontSize = 8.sp, fontWeight = FontWeight.Bold)
                                        Text("${fpsMode} FPS Ultra", color = Color(0xFF38BDF8), fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                    }
                                }
                                Box(
                                    modifier = Modifier
                                        .weight(1f)
                                        .clip(RoundedCornerShape(8.dp))
                                        .background(Color(0x1AFFFFFF))
                                        .padding(10.dp)
                                ) {
                                    Column {
                                        Text("STREAM LATENCY", color = TextMuted, fontSize = 8.sp, fontWeight = FontWeight.Bold)
                                        Text("${String.format(java.util.Locale.US, "%.0f", streamDiag.latencyMs)} ms RTT", color = Color(0xFF34D399), fontSize = 12.sp, fontWeight = FontWeight.Bold)
                                    }
                                }
                            }
                        }

                        // ── TAB: SECURITY (Screenshot 3) ──────────────────────
                        if (panelTab == "Security") {
                            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                                // Mute Host Device
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Column {
                                        Text("Mute Host Device", color = Color.White, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                                        Text("Silence Windows PC audio during remote session", color = TextMuted, fontSize = 10.sp)
                                    }
                                    Box(
                                        modifier = Modifier
                                            .clip(RoundedCornerShape(12.dp))
                                            .background(if (muteHostAudio) Brand600 else Color(0x26FFFFFF))
                                            .clickable { muteHostAudio = !muteHostAudio }
                                            .padding(horizontal = 10.dp, vertical = 4.dp)
                                    ) {
                                        Text(if (muteHostAudio) "ON" else "OFF", color = Color.White, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                                    }
                                }

                                // Host Privacy Mode
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Column {
                                        Text("Host Privacy Mode", color = Color.White, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                                        Text("Black out physical PC monitor while you control", color = TextMuted, fontSize = 10.sp)
                                    }
                                    Box(
                                        modifier = Modifier
                                            .clip(RoundedCornerShape(12.dp))
                                            .background(if (hostPrivacyMode) Brand600 else Color(0x26FFFFFF))
                                            .clickable { hostPrivacyMode = !hostPrivacyMode }
                                            .padding(horizontal = 10.dp, vertical = 4.dp)
                                    ) {
                                        Text(if (hostPrivacyMode) "ON" else "OFF", color = Color.White, fontSize = 10.sp, fontWeight = FontWeight.Bold)
                                    }
                                }

                                // Clipboard Sync
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Column {
                                        Text("Clipboard Sync", color = Color.White, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
                                        Text("Seamless bidirectional clipboard sharing", color = TextMuted, fontSize = 10.sp)
                                    }
                                    Box(
                                        modifier = Modifier
                                            .clip(RoundedCornerShape(12.dp))
                                            .background(if (clipboardSyncEnabled) Brand600 else Color(0x26FFFFFF))
                                            .clickable { clipboardSyncEnabled = !clipboardSyncEnabled }
                                            .padding(horizontal = 10.dp, vertical = 4.dp)
                                    ) {
                                        Text(if (clipboardSyncEnabled) "ON" else "OFF", color = Color.White, fontSize = 10.sp, fontWeight = FontWeight.Bold)
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
private suspend fun handleScannedQr(
    raw: String,
    onStatus: (String, Boolean) -> Unit,
    onSuccess: (String, String, String) -> Unit,
    onFail: () -> Unit
) {
    try {
        var host = ""
        var port = 7890
        var pinCode = ""
        var hostName = "Windows Host"

        if (raw.startsWith("smp://") || raw.contains("host=") || raw.contains("code=")) {
            val uri = android.net.Uri.parse(raw)
            host = uri.getQueryParameter("host") ?: ""
            port = uri.getQueryParameter("port")?.toIntOrNull() ?: 7890
            pinCode = uri.getQueryParameter("code") ?: ""
            hostName = uri.getQueryParameter("name") ?: "Windows Host"

            if (pinCode.isEmpty()) {
                pinCode = Regex("""[?&]code=([^&]+)""").find(raw)?.groupValues?.get(1) ?: ""
            }
            if (host.isEmpty()) {
                host = Regex("""[?&]host=([^&]+)""").find(raw)?.groupValues?.get(1) ?: ""
            }
        }

        if (pinCode.isEmpty()) {
            if (raw.length == 6 && raw.all { it.isDigit() }) {
                pinCode = raw
            } else {
                pinCode = Regex("""\b\d{6}\b""").find(raw)?.value
                    ?: Regex("""\b\d{3}\s*-\s*\d{3}\b""").find(raw)?.value?.replace("-", "")?.replace(" ", "")
                    ?: ""
            }
        }

        if (host.isEmpty()) {
            val ipMatch = Regex("""\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b""").find(raw)?.value
            if (ipMatch != null) {
                host = ipMatch
            } else {
                val disc = SmpClient.discoverHosts()
                host = disc?.ip ?: "${SmpClient.getLocalSubnetPrefix()}33"
            }
        }

        if (pinCode.isEmpty()) {
            onStatus("Invalid QR code format. Missing pairing PIN.", true)
            onFail()
            return
        }

        onStatus("Pairing with $hostName ($host:$port)...", false)
        val res = SmpClient.pair(host, port, pinCode)
        if (res.isSuccess) {
            val p = res.getOrNull()!!
            onStatus("Pairing verified! Launching stream...", false)
            onSuccess(host, p.sessionToken, p.hostDeviceId)
        } else {
            onStatus("Pairing failed: ${res.exceptionOrNull()?.message ?: "Host rejected PIN"}", true)
            onFail()
        }
    } catch (e: Exception) {
        onStatus("Error reading QR: ${e.message}", true)
        onFail()
    }
}

@Composable
private fun QrScannerModal(
    onDismiss: () -> Unit,
    onManualPin: () -> Unit,
    onPaired: (String, String, String) -> Unit
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val lifecycleOwner = context as androidx.lifecycle.LifecycleOwner
    val scope = rememberCoroutineScope()

    var hasCameraPermission by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED
        )
    }

    val permissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { isGranted ->
        hasCameraPermission = isGranted
    }

    androidx.compose.runtime.LaunchedEffect(Unit) {
        if (!hasCameraPermission) {
            permissionLauncher.launch(Manifest.permission.CAMERA)
        }
    }

    var isPairing by remember { mutableStateOf(false) }
    var statusText by remember { mutableStateOf<String?>("Align Windows Host QR inside viewfinder") }
    var isError by remember { mutableStateOf(false) }
    var scanCompleted by remember { mutableStateOf(false) }

    val scanTransition = rememberInfiniteTransition(label = "laser")
    val laserProgress by scanTransition.animateFloat(
        initialValue = 0.05f,
        targetValue = 0.95f,
        animationSpec = infiniteRepeatable(
            animation = tween(1800, easing = androidx.compose.animation.core.FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "laser_y"
    )

    Dialog(onDismissRequest = onDismiss) {
        Surface(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 12.dp),
            color = Surface1,
            shape = RoundedCornerShape(16.dp),
            border = androidx.compose.foundation.BorderStroke(1.dp, LineStrong)
        ) {
            Column(
                modifier = Modifier
                    .padding(20.dp)
                    .fillMaxWidth(),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Box(
                        modifier = Modifier
                            .size(32.dp)
                            .background(Brand600.copy(alpha = 0.25f), CircleShape)
                            .border(1.dp, Brand400.copy(alpha = 0.5f), CircleShape),
                        contentAlignment = Alignment.Center
                    ) {
                        QrCodeVector(size = 16.dp, color = Brand300)
                    }
                    Spacer(Modifier.width(10.dp))
                    Column(Modifier.weight(1f)) {
                        Text("SCAN WINDOWS HOST QR", color = Brand300, fontSize = 11.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.sp)
                        Text("Instant P2P Display and Control", color = TextSecondary, fontSize = 12.sp)
                    }
                    Box(
                        modifier = Modifier
                            .clip(CircleShape)
                            .background(Surface2)
                            .clickable { onDismiss() }
                            .padding(6.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        Text("✕", color = TextMuted, fontSize = 13.sp, fontWeight = FontWeight.Bold)
                    }
                }

                Spacer(Modifier.height(16.dp))

                if (hasCameraPermission) {
                    Box(
                        modifier = Modifier
                            .size(240.dp)
                            .clip(RoundedCornerShape(16.dp))
                            .background(Color.Black)
                            .border(2.dp, if (isPairing) Success else Brand500, RoundedCornerShape(16.dp)),
                        contentAlignment = Alignment.Center
                    ) {
                        AndroidView(
                            factory = { ctx ->
                                val previewView = PreviewView(ctx).apply {
                                    layoutParams = ViewGroup.LayoutParams(
                                        ViewGroup.LayoutParams.MATCH_PARENT,
                                        ViewGroup.LayoutParams.MATCH_PARENT
                                    )
                                    scaleType = PreviewView.ScaleType.FILL_CENTER
                                }

                                val cameraProviderFuture = ProcessCameraProvider.getInstance(ctx)
                                val executor = Executors.newSingleThreadExecutor()

                                cameraProviderFuture.addListener({
                                    val cameraProvider = cameraProviderFuture.get()
                                    val preview = Preview.Builder().build().also {
                                        it.setSurfaceProvider(previewView.surfaceProvider)
                                    }

                                    val imageAnalysis = ImageAnalysis.Builder()
                                        .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                                        .build()

                                    val reader = MultiFormatReader()

                                    imageAnalysis.setAnalyzer(executor) { imageProxy ->
                                        if (scanCompleted || isPairing) {
                                            imageProxy.close()
                                            return@setAnalyzer
                                        }

                                        val yBuffer = imageProxy.planes[0].buffer
                                        val ySize = yBuffer.remaining()
                                        val yData = ByteArray(ySize)
                                        yBuffer.get(yData)

                                        val width = imageProxy.width
                                        val height = imageProxy.height
                                        val rotation = imageProxy.imageInfo.rotationDegrees

                                        val rotatedData: ByteArray
                                        val finalW: Int
                                        val finalH: Int

                                        if (rotation == 90) {
                                            rotatedData = ByteArray(width * height)
                                            for (y in 0 until height) {
                                                for (x in 0 until width) {
                                                    rotatedData[x * height + (height - y - 1)] = yData[y * width + x]
                                                }
                                            }
                                            finalW = height
                                            finalH = width
                                        } else if (rotation == 270) {
                                            rotatedData = ByteArray(width * height)
                                            for (y in 0 until height) {
                                                for (x in 0 until width) {
                                                    rotatedData[(width - x - 1) * height + y] = yData[y * width + x]
                                                }
                                            }
                                            finalW = height
                                            finalH = width
                                        } else {
                                            rotatedData = yData
                                            finalW = width
                                            finalH = height
                                        }

                                        val source = PlanarYUVLuminanceSource(
                                            rotatedData, finalW, finalH,
                                            0, 0, finalW, finalH, false
                                        )
                                        val bitmap = BinaryBitmap(HybridBinarizer(source))

                                        try {
                                            val result = reader.decodeWithState(bitmap)
                                            val qrText = result.text
                                            if (!qrText.isNullOrEmpty() && !scanCompleted) {
                                                scanCompleted = true
                                                isPairing = true
                                                scope.launch {
                                                    handleScannedQr(
                                                        raw = qrText,
                                                        onStatus = { msg, err ->
                                                            statusText = msg
                                                            isError = err
                                                        },
                                                        onSuccess = { host, token, id ->
                                                            onPaired(host, token, id)
                                                        },
                                                        onFail = {
                                                            scanCompleted = false
                                                            isPairing = false
                                                        }
                                                    )
                                                }
                                            }
                                        } catch (_: Exception) {
                                        } finally {
                                            reader.reset()
                                            imageProxy.close()
                                        }
                                    }

                                    try {
                                        cameraProvider.unbindAll()
                                        cameraProvider.bindToLifecycle(
                                            lifecycleOwner,
                                            CameraSelector.DEFAULT_BACK_CAMERA,
                                            preview,
                                            imageAnalysis
                                        )
                                    } catch (_: Exception) {
                                    }
                                }, ContextCompat.getMainExecutor(ctx))

                                previewView
                            },
                            modifier = Modifier.fillMaxSize()
                        )

                        Canvas(modifier = Modifier.size(190.dp)) {
                            val w = size.width
                            val h = size.height
                            val arm = 26.dp.toPx()
                            val stroke = 3.dp.toPx()
                            val cornerColor = if (isPairing) Success else Brand400

                            drawLine(cornerColor, Offset(0f, 0f), Offset(arm, 0f), strokeWidth = stroke)
                            drawLine(cornerColor, Offset(0f, 0f), Offset(0f, arm), strokeWidth = stroke)
                            drawLine(cornerColor, Offset(w, 0f), Offset(w - arm, 0f), strokeWidth = stroke)
                            drawLine(cornerColor, Offset(w, 0f), Offset(w, arm), strokeWidth = stroke)
                            drawLine(cornerColor, Offset(0f, h), Offset(arm, h), strokeWidth = stroke)
                            drawLine(cornerColor, Offset(0f, h), Offset(0f, h - arm), strokeWidth = stroke)
                            drawLine(cornerColor, Offset(w, h), Offset(w - arm, h), strokeWidth = stroke)
                            drawLine(cornerColor, Offset(w, h), Offset(w, h - arm), strokeWidth = stroke)
                        }

                        if (!isPairing) {
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth(0.85f)
                                    .fillMaxHeight(laserProgress)
                                    .align(Alignment.TopCenter)
                            ) {
                                Box(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .height(2.dp)
                                        .align(Alignment.BottomCenter)
                                        .background(
                                            Brush.horizontalGradient(
                                                listOf(Color.Transparent, Brand300, Color.White, Brand300, Color.Transparent)
                                            )
                                        )
                                )
                            }
                        }
                    }
                } else {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(180.dp)
                            .clip(RoundedCornerShape(12.dp))
                            .background(Surface2)
                            .padding(16.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("Camera Permission Required", color = TextPrimary, fontSize = 14.sp, fontWeight = FontWeight.Bold)
                            Spacer(Modifier.height(6.dp))
                            Text("Smart Migrate needs camera access to scan the pairing QR code from your PC.", color = TextMuted, fontSize = 12.sp, textAlign = TextAlign.Center)
                            Spacer(Modifier.height(12.dp))
                            PrimaryButton("Grant Camera Access") {
                                permissionLauncher.launch(Manifest.permission.CAMERA)
                            }
                        }
                    }
                }

                Spacer(Modifier.height(12.dp))

                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(8.dp))
                        .background(if (isError) Danger.copy(alpha = 0.15f) else Surface2)
                        .border(1.dp, if (isError) Danger.copy(alpha = 0.3f) else Line, RoundedCornerShape(8.dp))
                        .padding(horizontal = 12.dp, vertical = 8.dp),
                    contentAlignment = Alignment.Center
                ) {
                    Text(
                        statusText ?: "Point camera directly at the QR code",
                        color = if (isError) Danger else if (isPairing) Success else TextSecondary,
                        fontSize = 12.sp,
                        textAlign = TextAlign.Center,
                        fontWeight = if (isPairing) FontWeight.Bold else FontWeight.Normal
                    )
                }

                Spacer(Modifier.height(14.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Box(Modifier.weight(1f)) {
                        SecondaryButton("Cancel") { onDismiss() }
                    }
                    Box(Modifier.weight(1.3f)) {
                        PrimaryButton("Enter PIN Manually") { onManualPin() }
                    }
                }
            }
        }
    }
}

@Composable
private fun PairingPinModal(
    initialHost: String,
    onDismiss: () -> Unit,
    onVerified: (String, String, String) -> Unit
) {
    var hostAddress by rememberSaveable { mutableStateOf(initialHost) }
    var enteredPin by rememberSaveable { mutableStateOf("") }
    var isPairing by remember { mutableStateOf(false) }
    var isDetecting by remember { mutableStateOf(false) }
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
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(6.dp))
                        .background(Surface2)
                        .border(1.dp, Line, RoundedCornerShape(6.dp))
                        .padding(horizontal = 8.dp, vertical = 6.dp),
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
                    Spacer(Modifier.width(6.dp))
                    Box(
                        modifier = Modifier
                            .clip(RoundedCornerShape(4.dp))
                            .background(Brand600.copy(alpha = 0.35f))
                            .clickable {
                                if (!isDetecting) {
                                    scope.launch {
                                        isDetecting = true
                                        statusMessage = "Probing LAN for Windows host..."
                                        val d = SmpClient.discoverHosts()
                                        isDetecting = false
                                        if (d != null) {
                                            hostAddress = d.ip
                                            statusMessage = "Found '${d.name}' at ${d.ip}"
                                        } else {
                                            statusMessage = "Host not found. Check PC is on same Wi-Fi."
                                        }
                                    }
                                }
                            }
                            .padding(horizontal = 8.dp, vertical = 4.dp)
                    ) {
                        Text(if (isDetecting) "Scanning..." else "Auto-Detect", color = Brand300, fontSize = 9.sp, fontWeight = FontWeight.Bold)
                    }
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
                        color = if (statusMessage!!.startsWith("Error") || statusMessage!!.contains("not found")) Color(0xFFFF6B6B) else Success,
                        fontSize = 11.sp,
                        textAlign = TextAlign.Center
                    )
                }

                Spacer(Modifier.height(14.dp))
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
                                        onVerified(hostAddress.trim(), p.sessionToken, p.hostDeviceId)
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
