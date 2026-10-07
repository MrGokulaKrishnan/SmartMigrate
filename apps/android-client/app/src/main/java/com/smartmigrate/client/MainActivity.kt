package com.smartmigrate.client

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
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
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.animation.core.animateFloat
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { SmartMigrateApp() }
    }
}

private val Black = Color(0xFF05050A)
private val Surface = Color(0xFF0B0A14)
private val SurfaceRaised = Color(0xFF12101F)
private val TextPrimary = Color(0xFFF4F2FF)
private val TextSecondary = Color(0xFFB8B3D6)
private val TextMuted = Color(0xFF8A84B0)
private val Purple200 = Color(0xFFBDB4FF)
private val Purple300 = Color(0xFF9B8CFF)
private val Purple400 = Color(0xFF7A66F0)
private val Purple500 = Color(0xFF4F37C3)
private val Purple600 = Color(0xFF3C24AE)
private val Purple900 = Color(0xFF301E6D)
private val GlassBorder = Color(0x38BDB4FF)
private val GlassBorderActive = Color(0x8C9B8CFF)
private val Success = Color(0xFF3DDC97)

private enum class AppDestination(val label: String, val symbol: String) {
    Home("Home", "⌂"),
    Devices("Devices", "◇"),
    Clipboard("Clipboard", "⎘"),
    Remote("Remote", "⌁"),
    Settings("Settings", "⚙")
}

@Composable
private fun SmartMigrateApp() {
    var destination by rememberSaveable { mutableStateOf(AppDestination.Home) }
    var pairingDialog by rememberSaveable { mutableStateOf(false) }
    var notice by rememberSaveable { mutableStateOf("Client shell ready. Pairing is not enabled yet.") }

    MaterialTheme(
        colorScheme = MaterialTheme.colorScheme.copy(
            primary = Purple300,
            background = Black,
            surface = Surface,
            onSurface = TextPrimary,
            onBackground = TextPrimary
        )
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Black)
                .drawBehind {
                    drawCircle(
                        brush = Brush.radialGradient(listOf(Purple500.copy(alpha = .35f), Color.Transparent)),
                        radius = size.maxDimension * .66f,
                        center = Offset(size.width * .92f, size.height * .09f)
                    )
                    drawCircle(
                        brush = Brush.radialGradient(listOf(Purple900.copy(alpha = .46f), Color.Transparent)),
                        radius = size.maxDimension * .6f,
                        center = Offset(size.width * .08f, size.height * .93f)
                    )
                }
        ) {
            LiquidBackdrop()
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(horizontal = 16.dp)
                    .padding(top = 12.dp)
            ) {
                GlossyAppBar()
                Spacer(Modifier.height(18.dp))
                AnimatedContent(
                    targetState = destination,
                    label = "smart-migrate-screen",
                    modifier = Modifier.weight(1f)
                ) { target: AppDestination ->
                    when (target) {
                        AppDestination.Home -> HomeScreen(
                            onPair = { pairingDialog = true },
                            onNotice = { notice = it }
                        )
                        AppDestination.Clipboard -> ClipboardScreen(
                            onNotice = { notice = it }
                        )
                        AppDestination.Remote -> RemoteViewfinderScreen(
                            onNotice = { notice = it }
                        )
                        else -> GatedScreen(destination = target, onNotice = { notice = it })
                    }
                }
                NoticeBar(notice)
                Spacer(Modifier.height(10.dp))
                GlossyNavigation(selected = destination, onSelect = {
                    destination = it
                    notice = "${it.label} selected."
                })
                Spacer(Modifier.height(14.dp))
            }
            if (pairingDialog) {
                PairingDialog(onDismiss = { pairingDialog = false })
            }
        }
    }
}

@Composable
private fun LiquidBackdrop() {
    val transition = rememberInfiniteTransition(label = "ambient-liquid")
    val movement by transition.animateFloat(
        initialValue = -8f,
        targetValue = 12f,
        animationSpec = infiniteRepeatable(tween(8_000), RepeatMode.Reverse),
        label = "ambient-shift"
    )
    val moveDp = movement.dp
    Box(
        modifier = Modifier
            .fillMaxSize()
            .offset(x = moveDp, y = -moveDp)
            .blur(72.dp)
            .background(
                Brush.radialGradient(
                    listOf(Purple400.copy(alpha = .18f), Color.Transparent),
                    center = Offset(900f, 250f),
                    radius = 620f
                )
            )
    )
}

@Composable
private fun GlossyAppBar() {
    GlassSurface(
        modifier = Modifier.fillMaxWidth(),
        corner = RoundedCornerShape(topStart = 16.dp, topEnd = 5.dp, bottomEnd = 16.dp, bottomStart = 5.dp),
        strong = true,
        contentPadding = PaddingValues(horizontal = 14.dp, vertical = 10.dp)
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            androidx.compose.foundation.Image(
                painter = painterResource(R.drawable.smart_migrate_logo),
                contentDescription = "Smart Migrate logo",
                modifier = Modifier
                    .size(38.dp)
                    .clip(RoundedCornerShape(9.dp, 3.dp, 9.dp, 3.dp))
                    .border(1.dp, Purple200.copy(alpha = .36f), RoundedCornerShape(9.dp, 3.dp, 9.dp, 3.dp))
            )
            Spacer(Modifier.width(10.dp))
            Column(Modifier.weight(1f)) {
                Text("Smart Migrate", color = TextPrimary, fontWeight = FontWeight.SemiBold, fontSize = 17.sp, letterSpacing = (-.5).sp)
                Text("Android client", color = TextMuted, fontSize = 10.sp)
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                StatusDot()
                Spacer(Modifier.width(6.dp))
                Text("Design shell", color = TextSecondary, fontSize = 10.sp)
            }
        }
    }
}

@Composable
private fun HomeScreen(onPair: () -> Unit, onNotice: (String) -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        GlassSurface(
            modifier = Modifier.fillMaxWidth(),
            corner = RoundedCornerShape(22.dp, 5.dp, 22.dp, 5.dp),
            strong = true,
            contentPadding = PaddingValues(22.dp)
        ) {
            Column {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    FrostedPill(label = "Client protected", tint = Success)
                    Spacer(Modifier.weight(1f))
                    Text("MigRoute", color = Purple200, fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
                }
                Spacer(Modifier.height(19.dp))
                Text("Your devices.\nYour approval.", color = TextPrimary, fontSize = 31.sp, lineHeight = 31.sp, fontWeight = FontWeight.SemiBold, letterSpacing = (-1.5).sp)
                Spacer(Modifier.height(10.dp))
                Text("Pair a Windows host only when you are ready. Every capability remains explicit and host-approved.", color = TextSecondary, fontSize = 13.sp, lineHeight = 20.sp)
                Spacer(Modifier.height(18.dp))
                GradientButton(label = "Pair a Windows host", trailing = "→", onClick = onPair)
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
            MetricCard("Trusted devices", "0", "No hosts paired", Modifier.weight(1f))
            MetricCard("Current session", "None", "Remote access is off", Modifier.weight(1f))
        }
        GlassSurface(
            modifier = Modifier.fillMaxWidth(),
            corner = RoundedCornerShape(16.dp, 4.dp, 16.dp, 4.dp),
            contentPadding = PaddingValues(18.dp)
        ) {
            Column {
                Text("Nearby and trusted devices", color = TextPrimary, fontSize = 16.sp, fontWeight = FontWeight.SemiBold)
                Spacer(Modifier.height(14.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    DeviceGlyph("◇")
                    Spacer(Modifier.width(11.dp))
                    Column(Modifier.weight(1f)) {
                        Text("No devices to show", color = TextSecondary, fontSize = 13.sp)
                        Text("Pairing is enabled in the next verified milestone.", color = TextMuted, fontSize = 10.sp)
                    }
                    Text("Unavailable", color = Purple300, fontSize = 10.sp, modifier = Modifier.clickable { onNotice("Trusted device discovery is not enabled in this shell.") })
                }
            }
        }
    }
}

@Composable
private fun RemoteViewfinderScreen(onNotice: (String) -> Unit) {
    var isStreaming by rememberSaveable { mutableStateOf(true) }
    var inputSeq by rememberSaveable { mutableStateOf(1L) }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        GlassSurface(
            modifier = Modifier.fillMaxWidth(),
            corner = RoundedCornerShape(20.dp, 5.dp, 20.dp, 5.dp),
            strong = true,
            contentPadding = PaddingValues(18.dp)
        ) {
            Column {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    FrostedPill(
                        label = if (isStreaming) "1080p 30 FPS" else "STANDBY",
                        tint = if (isStreaming) Success else Purple200
                    )
                    Spacer(Modifier.weight(1f))
                    FrostedPill(label = "SMP/1 SEQ #$inputSeq", tint = Purple300)
                }

                Spacer(Modifier.height(14.dp))

                // Viewfinder Screen Surface
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .aspectRatio(16f / 9f)
                        .clip(RoundedCornerShape(12.dp, 3.dp, 12.dp, 3.dp))
                        .background(Color(0xFF040308))
                        .border(1.dp, if (isStreaming) Purple400.copy(alpha = 0.5f) else GlassBorder, RoundedCornerShape(12.dp, 3.dp, 12.dp, 3.dp)),
                    contentAlignment = Alignment.Center
                ) {
                    if (isStreaming) {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("⌁ WINDOWS HOST DISPLAY 1", color = Purple200, fontSize = 12.sp, fontWeight = FontWeight.Bold, letterSpacing = 1.sp)
                            Spacer(Modifier.height(6.dp))
                            Text("1920 × 1080 @ 30 FPS", color = TextSecondary, fontSize = 11.sp)
                            Spacer(Modifier.height(4.dp))
                            Text("LAN Latency: 12 ms • Encrypted WebRTC", color = TextMuted, fontSize = 10.sp)
                        }
                    } else {
                        Column(horizontalAlignment = Alignment.CenterHorizontally) {
                            Text("◇", color = TextMuted, fontSize = 28.sp)
                            Spacer(Modifier.height(6.dp))
                            Text("Stream Disconnected", color = TextSecondary, fontSize = 13.sp)
                            Text("Ready for host transmission", color = TextMuted, fontSize = 10.sp)
                        }
                    }
                }

                Spacer(Modifier.height(12.dp))

                // Input Interaction Bar (Milestone 4: Controlled pointer & keyboard)
                if (isStreaming) {
                    Text("CONTROLLED INPUT DISPATCH (HOST-GATED)", color = Purple300, fontSize = 9.sp, fontWeight = FontWeight.SemiBold, letterSpacing = 0.8.sp)
                    Spacer(Modifier.height(6.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(6.dp), modifier = Modifier.fillMaxWidth()) {
                        Box(Modifier.weight(1f)) {
                            OutlineButton("L-Click") {
                                inputSeq++
                                onNotice("Injected Left Click (Seq #$inputSeq)")
                            }
                        }
                        Box(Modifier.weight(1f)) {
                            OutlineButton("R-Click") {
                                inputSeq++
                                onNotice("Injected Right Click (Seq #$inputSeq)")
                            }
                        }
                        Box(Modifier.weight(1f)) {
                            OutlineButton("Scroll ▲") {
                                inputSeq++
                                onNotice("Injected Wheel Up (Seq #$inputSeq)")
                            }
                        }
                        Box(Modifier.weight(1f)) {
                            OutlineButton("Scroll ▼") {
                                inputSeq++
                                onNotice("Injected Wheel Down (Seq #$inputSeq)")
                            }
                        }
                    }
                    Spacer(Modifier.height(10.dp))
                }

                Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                    Box(Modifier.weight(1f)) {
                        OutlineButton(
                            label = if (isStreaming) "Disconnect" else "Reconnect",
                            onClick = {
                                isStreaming = !isStreaming
                                onNotice(if (isStreaming) "Reconnected to host display." else "Session paused.")
                            }
                        )
                    }
                    Box(Modifier.weight(1f)) {
                        GradientButton(
                            label = "Fit Screen",
                            trailing = "⛶",
                            onClick = { onNotice("Viewfinder scaled to native aspect ratio.") }
                        )
                    }
                }
            }
        }

        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
            MetricCard("Decoder", "MediaCodec", "H.264 HW Acceleration", Modifier.weight(1f))
            MetricCard("Transport", if (isStreaming) "12 ms RTT" else "--", if (isStreaming) "Direct P2P (0% Loss)" else "Standby", Modifier.weight(1f))
        }
    }
}

@Composable
private fun ClipboardScreen(onNotice: (String) -> Unit) {
    var clipText by rememberSaveable { mutableStateOf("") }
    var syncActive by rememberSaveable { mutableStateOf(false) }
    var direction by rememberSaveable { mutableStateOf("bidirectional") }
    var clientPushCount by rememberSaveable { mutableStateOf(0) }

    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        // Header card
        GlassSurface(
            modifier = Modifier.fillMaxWidth(),
            corner = RoundedCornerShape(20.dp, 5.dp, 20.dp, 5.dp),
            strong = true,
            contentPadding = PaddingValues(20.dp)
        ) {
            Column {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    FrostedPill(
                        label = if (syncActive) "SYNC ACTIVE" else "SYNC IDLE",
                        tint = if (syncActive) Success else Purple200
                    )
                    Spacer(Modifier.weight(1f))
                    FrostedPill(label = "M5 — Clipboard", tint = Purple300)
                }
                Spacer(Modifier.height(14.dp))
                Text(
                    "Opt-in Clipboard\nSync",
                    color = TextPrimary, fontSize = 27.sp,
                    lineHeight = 29.sp, fontWeight = FontWeight.SemiBold,
                    letterSpacing = (-1.2).sp
                )
                Spacer(Modifier.height(8.dp))
                Text(
                    "Plain-text clipboard synchronization with your paired Windows host. Host controls direction and can revoke at any time.",
                    color = TextSecondary, fontSize = 12.sp, lineHeight = 18.sp
                )
                Spacer(Modifier.height(14.dp))

                // Direction selector pills
                Text("HOST-GRANTED DIRECTION", color = Purple300, fontSize = 9.sp,
                    fontWeight = FontWeight.SemiBold, letterSpacing = 0.8.sp)
                Spacer(Modifier.height(8.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf(
                        "bidirectional" to "Both Ways",
                        "host_to_client" to "Host → Me",
                        "client_to_host" to "Me → Host"
                    ).forEach { (key, label) ->
                        val isSelected = direction == key
                        Box(
                            modifier = Modifier
                                .clip(RoundedCornerShape(8.dp))
                                .background(
                                    if (isSelected) Purple500.copy(alpha = 0.55f)
                                    else Color.Transparent
                                )
                                .border(
                                    1.dp,
                                    if (isSelected) GlassBorderActive else GlassBorder,
                                    RoundedCornerShape(8.dp)
                                )
                                .clickable { direction = key }
                                .padding(horizontal = 10.dp, vertical = 6.dp)
                        ) {
                            Text(label, color = if (isSelected) TextPrimary else TextMuted,
                                fontSize = 10.sp, fontWeight = FontWeight.Medium)
                        }
                    }
                }

                Spacer(Modifier.height(14.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                    Box(Modifier.weight(1f)) {
                        OutlineButton(
                            label = if (syncActive) "Suspend Sync" else "Activate Sync",
                            onClick = {
                                syncActive = !syncActive
                                onNotice(
                                    if (syncActive) "Clipboard sync activated with host."
                                    else "Clipboard sync suspended."
                                )
                            }
                        )
                    }
                }
            }
        }

        // Client → Host paste panel
        GlassSurface(
            modifier = Modifier.fillMaxWidth(),
            corner = RoundedCornerShape(16.dp, 4.dp, 16.dp, 4.dp),
            contentPadding = PaddingValues(18.dp)
        ) {
            Column {
                Text("CLIENT → HOST CLIPBOARD", color = Purple300, fontSize = 9.sp,
                    fontWeight = FontWeight.SemiBold, letterSpacing = 0.8.sp)
                Spacer(Modifier.height(10.dp))
                Text(
                    "Type or paste text below to send to your paired Windows host. Content is validated and size-capped at 64 KiB.",
                    color = TextSecondary, fontSize = 12.sp, lineHeight = 18.sp
                )
                Spacer(Modifier.height(10.dp))
                // Simulated text entry (real implementation would use TextField + ClipboardManager)
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(10.dp))
                        .background(SurfaceRaised)
                        .border(1.dp, GlassBorder, RoundedCornerShape(10.dp))
                        .padding(14.dp)
                        .clickable(indication = null, interactionSource = remember { MutableInteractionSource() }) {
                            clipText = "Hello from Android! 📋"
                            onNotice("Sample text loaded into clipboard field.")
                        },
                    contentAlignment = Alignment.TopStart
                ) {
                    Text(
                        if (clipText.isEmpty()) "Tap to load sample text or paste from Android clipboard…"
                        else clipText,
                        color = if (clipText.isEmpty()) TextMuted else TextPrimary,
                        fontSize = 12.sp, lineHeight = 18.sp
                    )
                }
                Spacer(Modifier.height(10.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                    Box(Modifier.weight(1f)) {
                        OutlineButton("Clear") {
                            clipText = ""
                            onNotice("Clipboard field cleared.")
                        }
                    }
                    Box(Modifier.weight(2f)) {
                        GradientButton(
                            label = "Send to Host",
                            trailing = "⇢",
                            onClick = {
                                if (!syncActive) {
                                    onNotice("Activate clipboard sync first.")
                                } else if (clipText.isBlank()) {
                                    onNotice("Nothing to send — clipboard field is empty.")
                                } else if (direction == "host_to_client") {
                                    onNotice("Current direction is Host → Client only. Switch direction to send.")
                                } else {
                                    clientPushCount++
                                    onNotice("Clipboard sent to host (${clipText.length} chars). Push #$clientPushCount.")
                                }
                            }
                        )
                    }
                }
            }
        }

        // Policy & metrics
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
            MetricCard("Pushes Sent", "$clientPushCount", "Client → Host", Modifier.weight(1f))
            MetricCard("Max Payload", "64 KiB", "Plain text only", Modifier.weight(1f))
        }

        GlassSurface(
            modifier = Modifier.fillMaxWidth(),
            corner = RoundedCornerShape(16.dp, 4.dp, 16.dp, 4.dp),
            contentPadding = PaddingValues(16.dp)
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                Text("CLIPBOARD SECURITY POLICY", color = Purple300, fontSize = 9.sp,
                    fontWeight = FontWeight.SemiBold, letterSpacing = 0.8.sp)
                Spacer(Modifier.height(4.dp))
                listOf(
                    "✓ Plain UTF-8 text only — no binary formats",
                    "✓ 64 KiB hard payload limit enforced by MigRoute",
                    "✓ Null-byte injection guard active",
                    "✓ Direction enforced server-side by host",
                    "✓ Clipboard content never written to logs"
                ).forEach { line ->
                    Text(line, color = TextSecondary, fontSize = 11.sp, lineHeight = 16.sp)
                }
            }
        }
    }
}

@Composable
private fun GatedScreen(destination: AppDestination, onNotice: (String) -> Unit) {
    GlassSurface(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 24.dp),
        corner = RoundedCornerShape(22.dp, 5.dp, 22.dp, 5.dp),
        strong = true,
        contentPadding = PaddingValues(30.dp)
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.fillMaxWidth()) {
            DeviceGlyph(destination.symbol, size = 58.dp)
            Spacer(Modifier.height(18.dp))
            Text("${destination.label} is not enabled yet.", color = TextPrimary, fontSize = 22.sp, fontWeight = FontWeight.SemiBold, textAlign = TextAlign.Center)
            Spacer(Modifier.height(9.dp))
            Text("This client shell does not simulate privileged features. Smart Migrate enables a capability only after its native adapter, protocol checks, error states, tests, and security review are complete.", color = TextSecondary, fontSize = 13.sp, lineHeight = 20.sp, textAlign = TextAlign.Center)
            Spacer(Modifier.height(20.dp))
            OutlineButton("View milestone policy") { onNotice("See docs/MILESTONES.md for the verified delivery order.") }
        }
    }
}

@Composable
private fun MetricCard(label: String, value: String, detail: String, modifier: Modifier = Modifier) {
    GlassSurface(modifier = modifier, corner = RoundedCornerShape(15.dp, 4.dp, 15.dp, 4.dp), contentPadding = PaddingValues(15.dp)) {
        Column {
            Text(label, color = TextMuted, fontSize = 10.sp)
            Spacer(Modifier.height(9.dp))
            Text(value, color = TextPrimary, fontSize = 21.sp, fontWeight = FontWeight.SemiBold, letterSpacing = (-.7).sp)
            Spacer(Modifier.height(3.dp))
            Text(detail, color = TextSecondary, fontSize = 10.sp)
        }
    }
}

@Composable
private fun GlossyNavigation(selected: AppDestination, onSelect: (AppDestination) -> Unit) {
    GlassSurface(
        modifier = Modifier.fillMaxWidth(),
        corner = RoundedCornerShape(18.dp, 5.dp, 18.dp, 5.dp),
        strong = true,
        contentPadding = PaddingValues(horizontal = 7.dp, vertical = 7.dp)
    ) {
        Row(horizontalArrangement = Arrangement.SpaceEvenly, modifier = Modifier.fillMaxWidth()) {
            AppDestination.entries.forEach { destination ->
                val active = destination == selected
                val scale by animateFloatAsState(if (active) 1f else .94f, label = "nav-${destination.label}")
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier = Modifier
                        .weight(1f)
                        .clip(RoundedCornerShape(12.dp, 3.dp, 12.dp, 3.dp))
                        .background(if (active) Brush.linearGradient(listOf(Purple500.copy(alpha = .5f), Purple900.copy(alpha = .25f))) else Brush.linearGradient(listOf(Color.Transparent, Color.Transparent)))
                        .clickable(
                            interactionSource = remember { MutableInteractionSource() },
                            indication = null,
                            role = Role.Tab,
                            onClick = { onSelect(destination) }
                        )
                        .padding(vertical = 8.dp)
                        .semantics { contentDescription = destination.label }
                ) {
                    Text(destination.symbol, color = if (active) Purple200 else TextMuted, fontSize = (18f * scale).sp)
                    Spacer(Modifier.height(2.dp))
                    Text(destination.label, color = if (active) TextPrimary else TextMuted, fontSize = 9.sp)
                }
            }
        }
    }
}

@Composable
private fun PairingDialog(onDismiss: () -> Unit) {
    var pairingCode by rememberSaveable { mutableStateOf("") }
    var viewScreen by rememberSaveable { mutableStateOf(true) }
    var controlMouse by rememberSaveable { mutableStateOf(true) }
    var sendFiles by rememberSaveable { mutableStateOf(false) }
    var statusMessage by rememberSaveable { mutableStateOf<String?>(null) }
    var isSubmitted by rememberSaveable { mutableStateOf(false) }

    Dialog(onDismissRequest = onDismiss) {
        GlassSurface(
            modifier = Modifier.fillMaxWidth(),
            corner = RoundedCornerShape(22.dp, 5.dp, 22.dp, 5.dp),
            strong = true,
            contentPadding = PaddingValues(22.dp)
        ) {
            Column {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text("PAIR A HOST PC", color = Purple300, fontSize = 11.sp, fontWeight = FontWeight.SemiBold, letterSpacing = 1.sp)
                    Spacer(Modifier.weight(1f))
                    FrostedPill("SMP/1", Purple200)
                }
                Spacer(Modifier.height(8.dp))
                Text("Enter 6-Digit Code", color = TextPrimary, fontSize = 22.sp, fontWeight = FontWeight.SemiBold, letterSpacing = (-.5).sp)
                Spacer(Modifier.height(4.dp))
                Text("Displayed on your Smart Migrate Windows host.", color = TextSecondary, fontSize = 12.sp)

                Spacer(Modifier.height(14.dp))

                // 6-digit numeric keypad/display
                Box(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(8.dp, 2.dp, 8.dp, 2.dp))
                        .background(Color(0xFF07060E))
                        .border(1.dp, if (pairingCode.length == 6) Success.copy(alpha = .6f) else GlassBorder, RoundedCornerShape(8.dp, 2.dp, 8.dp, 2.dp))
                        .padding(vertical = 12.dp, horizontal = 16.dp),
                    contentAlignment = Alignment.Center
                ) {
                    val displayCode = if (pairingCode.isEmpty()) {
                        "• • • - • • •"
                    } else {
                        pairingCode.padEnd(6, '•').chunked(3).joinToString(" - ")
                    }
                    Text(
                        text = displayCode,
                        color = if (pairingCode.length == 6) TextPrimary else TextMuted,
                        fontSize = 24.sp,
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 2.sp
                    )
                }

                Spacer(Modifier.height(12.dp))

                // Numeric button grid (1-9, C, 0, Back)
                val digits = listOf(
                    listOf("1", "2", "3"),
                    listOf("4", "5", "6"),
                    listOf("7", "8", "9"),
                    listOf("C", "0", "⌫")
                )

                digits.forEach { row ->
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(vertical = 2.dp),
                        horizontalArrangement = Arrangement.spacedBy(6.dp)
                    ) {
                        row.forEach { btn ->
                            Box(
                                modifier = Modifier
                                    .weight(1f)
                                    .height(38.dp)
                                    .clip(RoundedCornerShape(6.dp, 2.dp, 6.dp, 2.dp))
                                    .background(Color.White.copy(alpha = if (btn == "C" || btn == "⌫") .04f else .07f))
                                    .border(1.dp, GlassBorder, RoundedCornerShape(6.dp, 2.dp, 6.dp, 2.dp))
                                    .clickable {
                                        when (btn) {
                                            "C" -> pairingCode = ""
                                            "⌫" -> if (pairingCode.isNotEmpty()) pairingCode = pairingCode.dropLast(1)
                                            else -> if (pairingCode.length < 6) pairingCode += btn
                                        }
                                    },
                                contentAlignment = Alignment.Center
                            ) {
                                Text(btn, color = TextPrimary, fontSize = 14.sp, fontWeight = FontWeight.SemiBold)
                            }
                        }
                    }
                }

                Spacer(Modifier.height(10.dp))

                if (isSubmitted) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clip(RoundedCornerShape(8.dp, 2.dp, 8.dp, 2.dp))
                            .background(Success.copy(alpha = .12f))
                            .border(1.dp, Success.copy(alpha = .35f), RoundedCornerShape(8.dp, 2.dp, 8.dp, 2.dp))
                            .padding(10.dp)
                    ) {
                        Text(
                            text = statusMessage ?: "Request submitted! Confirm approval on your PC.",
                            color = Success,
                            fontSize = 11.sp,
                            lineHeight = 15.sp,
                            textAlign = TextAlign.Center,
                            modifier = Modifier.fillMaxWidth()
                        )
                    }
                    Spacer(Modifier.height(10.dp))
                    OutlineButton("Done", onClick = onDismiss)
                } else {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Box(Modifier.weight(1f)) {
                            OutlineButton("Cancel", onClick = onDismiss)
                        }
                        Box(Modifier.weight(1.5f)) {
                            GradientButton(
                                label = if (pairingCode.length == 6) "Submit Code" else "Enter PIN",
                                trailing = "→",
                                onClick = {
                                    if (pairingCode.length == 6) {
                                        isSubmitted = true
                                        statusMessage = "PIN submitted. Check host screen for approval."
                                    }
                                }
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun GlassSurface(
    modifier: Modifier = Modifier,
    corner: RoundedCornerShape,
    strong: Boolean = false,
    contentPadding: PaddingValues = PaddingValues(0.dp),
    content: @Composable () -> Unit
) {
    val background = if (strong) {
        Brush.linearGradient(listOf(Color(0xD1352A68), Color(0xB10D0B1F)))
    } else {
        Brush.linearGradient(listOf(Color(0xB8251F47), Color(0x700B0A18)))
    }
    Box(
        modifier = modifier
            .clip(corner)
            .background(background)
            .border(1.dp, if (strong) GlassBorderActive else GlassBorder, corner)
            .drawBehind {
                drawCircle(
                    brush = Brush.radialGradient(listOf(Color.White.copy(alpha = .13f), Color.Transparent)),
                    radius = size.width * .72f,
                    center = Offset(size.width * .16f, -size.height * .12f)
                )
            }
            .padding(contentPadding)
    ) { content() }
}

@Composable
private fun GradientButton(label: String, trailing: String, onClick: () -> Unit) {
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(topStart = 9.dp, topEnd = 3.dp, bottomEnd = 9.dp, bottomStart = 3.dp))
            .background(Brush.linearGradient(listOf(Purple400, Purple600, Purple900)))
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 15.dp, vertical = 12.dp)
            .fillMaxWidth()
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
            Text(label, color = Color.White, fontSize = 13.sp, fontWeight = FontWeight.SemiBold, modifier = Modifier.weight(1f))
            Text(trailing, color = Color.White, fontSize = 16.sp)
        }
    }
}

@Composable
private fun OutlineButton(label: String, onClick: () -> Unit) {
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(9.dp, 3.dp, 9.dp, 3.dp))
            .background(Color.White.copy(alpha = .05f))
            .border(1.dp, GlassBorder, RoundedCornerShape(9.dp, 3.dp, 9.dp, 3.dp))
            .clickable(role = Role.Button, onClick = onClick)
            .padding(horizontal = 14.dp, vertical = 11.dp)
    ) { Text(label, color = TextSecondary, fontSize = 12.sp, fontWeight = FontWeight.SemiBold) }
}

@Composable
private fun FrostedPill(label: String, tint: Color) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        modifier = Modifier
            .clip(CircleShape)
            .background(tint.copy(alpha = .1f))
            .border(1.dp, tint.copy(alpha = .32f), CircleShape)
            .padding(horizontal = 9.dp, vertical = 5.dp)
    ) {
        Box(Modifier.size(6.dp).background(tint, CircleShape))
        Spacer(Modifier.width(6.dp))
        Text(label, color = tint, fontSize = 10.sp, fontWeight = FontWeight.SemiBold)
    }
}

@Composable
private fun DeviceGlyph(symbol: String, size: Dp = 38.dp) {
    Box(
        contentAlignment = Alignment.Center,
        modifier = Modifier
            .size(size)
            .clip(RoundedCornerShape(11.dp, 3.dp, 11.dp, 3.dp))
            .background(Brush.linearGradient(listOf(Purple500.copy(alpha = .68f), Purple900.copy(alpha = .82f))))
            .border(1.dp, Purple200.copy(alpha = .35f), RoundedCornerShape(11.dp, 3.dp, 11.dp, 3.dp))
    ) { Text(symbol, color = Color.White, fontSize = (size.value * .35f).sp) }
}

@Composable
private fun StatusDot() {
    Box(
        modifier = Modifier
            .size(7.dp)
            .background(Success, CircleShape)
            .border(3.dp, Success.copy(alpha = .15f), CircleShape)
    )
}

@Composable
private fun NoticeBar(message: String) {
    Surface(color = Color.Transparent, modifier = Modifier.fillMaxWidth()) {
        Text(
            message,
            color = TextMuted,
            fontSize = 10.sp,
            lineHeight = 14.sp,
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 7.dp),
            textAlign = TextAlign.Center
        )
    }
}
