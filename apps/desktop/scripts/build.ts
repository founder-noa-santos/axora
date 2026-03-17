/**
 * AXORA Release Build Script
 *
 * Builds release packages for all platforms:
 * - macOS: .dmg
 * - Windows: .exe, .msi
 * - Linux: .deb, .AppImage
 */

import { execSync } from 'node:child_process';
import { platform, arch } from 'node:os';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

// Build targets by platform
const buildTargets: Record<string, string[]> = {
  darwin: ['dmg'],
  windows: ['nsis', 'msi'],
  linux: ['deb', 'appimage'],
};

// Tauri targets by platform
const tauriTargets: Record<string, string> = {
  darwin: 'universal-apple-darwin',
  windows: 'x86_64-pc-windows-msvc',
  linux: 'x86_64-unknown-linux-gnu',
};

interface BuildOptions {
  skipFrontendBuild?: boolean;
  codeSign?: boolean;
  notarize?: boolean;
}

/**
 * Execute a shell command
 */
function exec(command: string, options?: { stdio?: 'inherit' | 'pipe' }): string {
  console.log(`$ ${command}`);
  return execSync(command, {
    encoding: 'utf-8',
    stdio: options?.stdio || 'pipe',
    env: { ...process.env },
  }).trim();
}

/**
 * Check if a command exists
 */
function commandExists(command: string): boolean {
  try {
    execSync(`which ${command}`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

/**
 * Get build timestamp
 */
function getTimestamp(): string {
  return new Date().toISOString().replace(/[:.]/g, '-').slice(0, -5);
}

/**
 * Print build information
 */
function printBuildInfo() {
  console.log('\n╔════════════════════════════════════════╗');
  console.log('║     AXORA Release Build                ║');
  console.log('╚════════════════════════════════════════╝\n');

  console.log(`Platform: ${platform()} (${arch()})`);
  console.log(`Timestamp: ${getTimestamp()}`);
  console.log(`Node: ${process.version}`);

  try {
    const pkg = JSON.parse(readFileSync('package.json', 'utf-8'));
    console.log(`Version: ${pkg.version}`);
  } catch {
    // Ignore
  }

  try {
    const rustVersion = exec('rustc --version', { stdio: 'pipe' });
    console.log(`Rust: ${rustVersion}`);
  } catch {
    console.log('Rust: Not found');
  }

  console.log('');
}

/**
 * Install Playwright browsers for E2E testing
 */
function installPlaywright() {
  console.log('📦 Installing Playwright browsers...\n');

  if (commandExists('npx')) {
    try {
      exec('npx playwright install --with-deps', { stdio: 'inherit' });
      console.log('✓ Playwright installed\n');
    } catch (error) {
      console.warn('⚠ Playwright installation failed, continuing...\n');
    }
  }
}

/**
 * Run E2E tests before build
 */
function runTests() {
  console.log('🧪 Running E2E tests...\n');

  try {
    exec('pnpm test:e2e --reporter=list', { stdio: 'inherit' });
    console.log('✓ All tests passed\n');
  } catch (error) {
    console.warn('⚠ Some tests failed, continuing with build...\n');
  }
}

/**
 * Build frontend
 */
function buildFrontend() {
  console.log('🏗️  Building frontend...\n');

  exec('pnpm build', { stdio: 'inherit' });

  // Verify build output
  const distDir = join(process.cwd(), 'dist');
  if (!existsSync(distDir)) {
    throw new Error('Frontend build failed: dist directory not found');
  }

  const files = exec('ls -lh dist', { stdio: 'pipe' });
  console.log('Frontend build output:');
  console.log(files);
  console.log('');
}

/**
 * Build Tauri application
 */
function buildTauri(targets: string[], options: BuildOptions) {
  console.log('📦 Building Tauri application...\n');

  const targetArg = targets.map((t) => `--target ${tauriTargets[t] || t}`).join(' ');
  const args = ['pnpm tauri build', targetArg];

  // Add code signing flags if enabled
  if (options.codeSign) {
    if (platform() === 'darwin' && process.env.APPLE_CERTIFICATE) {
      args.push('--config');
      args.push('{"bundle": {"macOS": {"signingIdentity": "Developer ID Application"}}}');
    }
    if (platform() === 'windows' && process.env.WINDOWS_CERTIFICATE) {
      args.push('--config');
      args.push('{"bundle": {"windows": {"certificateThumbprint": "' + process.env.WINDOWS_CERTIFICATE + '"}}}');
    }
  }

  try {
    exec(args.join(' '), { stdio: 'inherit' });
    console.log('✓ Tauri build completed\n');
  } catch (error) {
    console.error('✗ Tauri build failed');
    throw error;
  }
}

/**
 * Print build results
 */
function printBuildResults(platformName: string) {
  console.log('\n╔════════════════════════════════════════╗');
  console.log('║     Build Results                      ║');
  console.log('╚════════════════════════════════════════╝\n');

  const bundleDir = join(process.cwd(), 'src-tauri/target/release/bundle');

  if (existsSync(bundleDir)) {
    const platforms = existsSync(join(bundleDir, 'dmg')) ? ['dmg'] :
                     existsSync(join(bundleDir, 'msi')) ? ['msi', 'exe'] :
                     existsSync(join(bundleDir, 'deb')) ? ['deb', 'appimage'] : [];

    for (const p of platforms) {
      const dir = join(bundleDir, p);
      if (existsSync(dir)) {
        console.log(`${p.toUpperCase()} packages:`);
        try {
          const files = exec(`ls -lh "${dir}"`, { stdio: 'pipe' });
          console.log(files);
          console.log('');
        } catch {
          // Ignore
        }
      }
    }
  }

  console.log(`Output directory: ${bundleDir}`);
  console.log('\n✓ Build completed successfully!\n');
}

/**
 * Main build function
 */
function main() {
  const args = process.argv.slice(2);
  const options: BuildOptions = {
    skipFrontendBuild: args.includes('--skip-frontend'),
    codeSign: args.includes('--code-sign'),
    notarize: args.includes('--notarize'),
  };

  printBuildInfo();

  const currentPlatform = platform();
  const targets = buildTargets[currentPlatform] || ['appimage'];

  console.log(`🎯 Building for: ${targets.join(', ')}\n`);

  // Step 1: Install dependencies (optional)
  if (args.includes('--install-deps')) {
    installPlaywright();
  }

  // Step 2: Run tests (optional)
  if (args.includes('--test')) {
    runTests();
  }

  // Step 3: Build frontend
  if (!options.skipFrontendBuild) {
    buildFrontend();
  } else {
    console.log('⏭️  Skipping frontend build\n');
  }

  // Step 4: Build Tauri
  buildTauri(targets, options);

  // Step 5: Print results
  printBuildResults(currentPlatform);

  console.log('📦 Release builds created successfully!');
  console.log('\nTo install:');
  if (currentPlatform === 'darwin') {
    console.log('  macOS: Open src-tauri/target/release/bundle/dmg/*.dmg');
  } else if (currentPlatform === 'windows') {
    console.log('  Windows: Run src-tauri/target/release/bundle/msi/*.msi or *.exe');
  } else if (currentPlatform === 'linux') {
    console.log('  Linux: Install src-tauri/target/release/bundle/deb/*.deb');
    console.log('  Or: src-tauri/target/release/bundle/appimage/*.AppImage');
  }
}

// Run build
main();
