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
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.drawCircle
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
                ) { selected ->
                    when (selected) {
                        AppDestination.Home -> HomeScreen(
                            onPair = { pairingDialog = true },
                            onNotice = { notice = it }
                        )
                        else -> GatedScreen(destination = selected, onNotice = { notice = it })
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
    Box(
        modifier = Modifier
            .fillMaxSize()
            .offset(x = movement.dp, y = (-movement).dp)
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
    Dialog(onDismissRequest = onDismiss) {
        GlassSurface(
            modifier = Modifier.fillMaxWidth(),
            corner = RoundedCornerShape(22.dp, 5.dp, 22.dp, 5.dp),
            strong = true,
            contentPadding = PaddingValues(25.dp)
        ) {
            Column {
                Text("Pair a device", color = Purple300, fontSize = 11.sp, fontWeight = FontWeight.SemiBold)
                Spacer(Modifier.height(10.dp))
                Text("Pairing is protected by design.", color = TextPrimary, fontSize = 23.sp, fontWeight = FontWeight.SemiBold, lineHeight = 26.sp)
                Spacer(Modifier.height(10.dp))
                Text("QR and numeric pairing will appear after the trusted-pairing milestone. This client never displays a fake code or claims a connection.", color = TextSecondary, fontSize = 13.sp, lineHeight = 20.sp)
                Spacer(Modifier.height(20.dp))
                GradientButton(label = "Got it", trailing = "✓", onClick = onDismiss)
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
