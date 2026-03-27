const sharp = require('sharp');
const path = require('path');
const fs = require('fs');

const RESOURCES_DIR = path.join(__dirname, '..', 'resources');

// Modern PingPal icon - smaller centered bell with black background
const createIconSvg = (size) => `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}">
  <defs>
    <linearGradient id="bellGrad" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" style="stop-color:#5cb97a"/>
      <stop offset="100%" style="stop-color:#3d8a5a"/>
    </linearGradient>
    <linearGradient id="pingGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" style="stop-color:#ff6b6b"/>
      <stop offset="100%" style="stop-color:#ee5a5a"/>
    </linearGradient>
    <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="${size * 0.01}" stdDeviation="${size * 0.02}" flood-color="#000" flood-opacity="0.5"/>
    </filter>
    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
      <feGaussianBlur stdDeviation="${size * 0.01}" result="coloredBlur"/>
      <feMerge>
        <feMergeNode in="coloredBlur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
  </defs>

  <!-- Solid black background with rounded corners -->
  <rect x="0" y="0" width="${size}" height="${size}" rx="${size * 0.22}" ry="${size * 0.22}" fill="#000000"/>

  <!-- Centered bell (scaled to 50% and centered) -->
  <g transform="translate(${size * 0.25}, ${size * 0.22}) scale(0.5)" filter="url(#shadow)">
    <!-- Bell body -->
    <path d="M${size * 0.5} ${size * 0.15}
             c${size * -0.2} 0 ${size * -0.35} ${size * 0.15} ${size * -0.35} ${size * 0.35}
             v${size * 0.2}
             l${size * -0.08} ${size * 0.08}
             v${size * 0.08}
             h${size * 0.86}
             v${size * -0.08}
             l${size * -0.08} ${size * -0.08}
             v${size * -0.2}
             c0 ${size * -0.2} ${size * -0.15} ${size * -0.35} ${size * -0.35} ${size * -0.35}z"
          fill="url(#bellGrad)"/>

    <!-- Bell clapper/bottom -->
    <ellipse cx="${size * 0.5}" cy="${size * 0.92}" rx="${size * 0.1}" ry="${size * 0.06}"
             fill="url(#bellGrad)"/>
  </g>

  <!-- Notification ping dot (positioned relative to bell) -->
  <g filter="url(#glow)">
    <circle cx="${size * 0.68}" cy="${size * 0.32}" r="${size * 0.07}" fill="url(#pingGrad)"/>
  </g>

  <!-- Ping wave -->
  <circle cx="${size * 0.68}" cy="${size * 0.32}" r="${size * 0.1}"
          fill="none" stroke="#ff6b6b" stroke-width="${size * 0.012}" opacity="0.4"/>
</svg>
`;

// Tray icon - simpler, works well at small sizes
const createTrayIconSvg = (size) => `
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${size} ${size}">
  <defs>
    <linearGradient id="trayBell" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" style="stop-color:#ffffff"/>
      <stop offset="100%" style="stop-color:#e0e0e0"/>
    </linearGradient>
  </defs>

  <!-- Bell -->
  <path d="M${size * 0.5} ${size * 0.1}
           c${size * -0.25} 0 ${size * -0.35} ${size * 0.2} ${size * -0.35} ${size * 0.35}
           v${size * 0.2}
           l${size * -0.05} ${size * 0.08}
           v${size * 0.07}
           h${size * 0.8}
           v${size * -0.07}
           l${size * -0.05} ${size * -0.08}
           v${size * -0.2}
           c0 ${size * -0.15} ${size * -0.1} ${size * -0.35} ${size * -0.35} ${size * -0.35}z"
        fill="url(#trayBell)"/>

  <!-- Bell clapper -->
  <ellipse cx="${size * 0.5}" cy="${size * 0.88}" rx="${size * 0.1}" ry="${size * 0.06}"
           fill="url(#trayBell)"/>

  <!-- Notification dot -->
  <circle cx="${size * 0.75}" cy="${size * 0.25}" r="${size * 0.12}" fill="#4a9f6e"/>
</svg>
`;

async function generateIcons() {
  console.log('Generating PingPal icons...');

  // Create resources directory if needed
  if (!fs.existsSync(RESOURCES_DIR)) {
    fs.mkdirSync(RESOURCES_DIR, { recursive: true });
  }

  // Generate main icon at 1024x1024
  const mainIconSvg = Buffer.from(createIconSvg(1024));

  // Save as PNG for Linux and as source for other formats
  await sharp(mainIconSvg)
    .resize(512, 512)
    .png()
    .toFile(path.join(RESOURCES_DIR, 'icon.png'));
  console.log('Created icon.png (512x512)');

  // Create 1024x1024 for high-res
  await sharp(mainIconSvg)
    .resize(1024, 1024)
    .png()
    .toFile(path.join(RESOURCES_DIR, 'icon-1024.png'));
  console.log('Created icon-1024.png');

  // Generate iconset for macOS
  const iconsetDir = path.join(RESOURCES_DIR, 'icon.iconset');
  if (!fs.existsSync(iconsetDir)) {
    fs.mkdirSync(iconsetDir, { recursive: true });
  }

  const sizes = [16, 32, 64, 128, 256, 512, 1024];
  for (const size of sizes) {
    await sharp(mainIconSvg)
      .resize(size, size)
      .png()
      .toFile(path.join(iconsetDir, `icon_${size}x${size}.png`));

    // @2x versions
    if (size <= 512) {
      await sharp(mainIconSvg)
        .resize(size * 2, size * 2)
        .png()
        .toFile(path.join(iconsetDir, `icon_${size}x${size}@2x.png`));
    }
  }
  console.log('Created macOS iconset');

  // Generate tray icon
  const trayIconSvg = Buffer.from(createTrayIconSvg(32));
  await sharp(trayIconSvg)
    .resize(22, 22)  // Standard tray icon size
    .png()
    .toFile(path.join(RESOURCES_DIR, 'tray-icon.png'));
  console.log('Created tray-icon.png');

  // Create @2x tray icon for retina
  await sharp(Buffer.from(createTrayIconSvg(64)))
    .resize(44, 44)
    .png()
    .toFile(path.join(RESOURCES_DIR, 'tray-icon@2x.png'));
  console.log('Created tray-icon@2x.png');

  // Generate Windows ICO sizes
  const icoSizes = [16, 24, 32, 48, 64, 128, 256];
  for (const size of icoSizes) {
    await sharp(mainIconSvg)
      .resize(size, size)
      .png()
      .toFile(path.join(RESOURCES_DIR, `icon-${size}.png`));
  }
  console.log('Created Windows icon sizes');

  console.log('\nIcon generation complete!');
  console.log('Run these commands to finalize:');
  console.log('  iconutil -c icns resources/icon.iconset -o resources/icon.icns');
  console.log('  # For Windows .ico, use an online converter or install png2ico');
}

generateIcons().catch(console.error);
