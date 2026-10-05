// Deterministic SVG QR-style visual code generator for Smart Migrate SMP/1 payloads.

export function QRCodeSvg({ value }: { value: string }) {
  // Deterministic 25x25 cell matrix based on string hash and markers
  const size = 25;
  const cells: boolean[][] = Array.from({ length: size }, () => Array(size).fill(false));

  // Finder pattern helper (7x7 box with 3x3 inner square)
  const drawFinder = (startX: number, startY: number) => {
    for (let y = 0; y < 7; y++) {
      for (let x = 0; x < 7; x++) {
        const isBorder = y === 0 || y === 6 || x === 0 || x === 6;
        const isCenter = y >= 2 && y <= 4 && x >= 2 && x <= 4;
        cells[startY + y][startX + x] = isBorder || isCenter;
      }
    }
  };

  drawFinder(0, 0); // Top-left
  drawFinder(size - 7, 0); // Top-right
  drawFinder(0, size - 7); // Bottom-left

  // Timing patterns
  for (let i = 8; i < size - 8; i++) {
    cells[6][i] = i % 2 === 0;
    cells[i][6] = i % 2 === 0;
  }

  // Hash-based data modules
  let hash = 0;
  for (let i = 0; i < value.length; i++) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0;
  }

  let lfsr = hash || 123456789;
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      // Don't overwrite finders or timing
      const inTopLeft = x < 8 && y < 8;
      const inTopRight = x >= size - 8 && y < 8;
      const inBottomLeft = x < 8 && y >= size - 8;
      const inTiming = x === 6 || y === 6;

      if (!inTopLeft && !inTopRight && !inBottomLeft && !inTiming) {
        lfsr = (lfsr * 1664525 + 1013904223) >>> 0;
        cells[y][x] = (lfsr & 1) === 1;
      }
    }
  }

  return (
    <svg viewBox={`0 0 ${size} ${size}`} width="100%" height="100%" shapeRendering="crispEdges">
      <rect width={size} height={size} fill="#ffffff" />
      {cells.map((row, y) =>
        row.map((active, x) =>
          active ? <rect key={`${x}-${y}`} x={x} y={y} width={1} height={1} fill="#140a2b" /> : null
        )
      )}
    </svg>
  );
}
