#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const https = require('https');
const { execSync } = require('child_process');
const { createWriteStream, createReadStream } = require('fs');
const { pipeline } = require('stream');
const { promisify } = require('util');

const pipelineAsync = promisify(pipeline);

// Package configuration
const PACKAGE_NAME = 'crowdcontrol';
const GITHUB_REPO = 'wadefletch/crowdcontrol';

// Platform and architecture detection
function getPlatform() {
  const platform = process.platform;
  const arch = process.arch;
  
  // Map Node.js platform/arch to Rust target triples
  const targets = {
    'darwin-x64': 'x86_64-apple-darwin',
    'darwin-arm64': 'aarch64-apple-darwin',
    'linux-x64': 'x86_64-unknown-linux-gnu',
    'linux-arm64': 'aarch64-unknown-linux-gnu',
    'win32-x64': 'x86_64-pc-windows-msvc'
  };
  
  const key = `${platform}-${arch}`;
  const target = targets[key];
  
  if (!target) {
    throw new Error(`Unsupported platform: ${platform}-${arch}`);
  }
  
  return {
    target,
    platform,
    arch,
    isWindows: platform === 'win32',
    extension: platform === 'win32' ? '.exe' : ''
  };
}

// Get the latest release version
async function getLatestVersion() {
  return new Promise((resolve, reject) => {
    const url = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
    
    https.get(url, {
      headers: {
        'User-Agent': 'npm-install-script'
      }
    }, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          const release = JSON.parse(data);
          resolve(release.tag_name);
        } catch (err) {
          reject(new Error(`Failed to parse release data: ${err.message}`));
        }
      });
    }).on('error', reject);
  });
}

// Download and extract binary
async function downloadBinary(version, platformInfo) {
  const { target, isWindows, extension } = platformInfo;
  const binDir = path.join(__dirname, 'bin');
  const binPath = path.join(binDir, `${PACKAGE_NAME}${extension}`);
  
  // Create bin directory
  if (!fs.existsSync(binDir)) {
    fs.mkdirSync(binDir, { recursive: true });
  }
  
  // Construct download URL
  const archiveExt = isWindows ? 'zip' : 'tar.gz';
  const archiveName = `${PACKAGE_NAME}-${target}.${archiveExt}`;
  const downloadUrl = `https://github.com/${GITHUB_REPO}/releases/download/${version}/${archiveName}`;
  
  console.log(`Downloading ${PACKAGE_NAME} ${version} for ${target}...`);
  console.log(`URL: ${downloadUrl}`);
  
  return new Promise((resolve, reject) => {
    const tempPath = path.join(__dirname, archiveName);
    const file = createWriteStream(tempPath);
    
    https.get(downloadUrl, (response) => {
      if (response.statusCode !== 200) {
        reject(new Error(`Download failed with status ${response.statusCode}`));
        return;
      }
      
      pipelineAsync(response, file)
        .then(() => {
          console.log('Download completed, extracting...');
          
          try {
            // Extract archive
            if (isWindows) {
              // For Windows, we need to handle ZIP extraction
              // This is a simple approach - in production you might want a proper ZIP library
              execSync(`powershell -command "Expand-Archive -Path '${tempPath}' -DestinationPath '${__dirname}' -Force"`, { stdio: 'inherit' });
            } else {
              execSync(`tar -xzf "${tempPath}" -C "${__dirname}"`, { stdio: 'inherit' });
            }
            
            // Make binary executable on Unix systems
            if (!isWindows) {
              fs.chmodSync(binPath, 0o755);
            }
            
            // Clean up archive
            fs.unlinkSync(tempPath);
            
            console.log(`✅ ${PACKAGE_NAME} installed successfully!`);
            resolve();
          } catch (err) {
            reject(new Error(`Extraction failed: ${err.message}`));
          }
        })
        .catch(reject);
    }).on('error', reject);
  });
}

// Main installation function
async function install() {
  try {
    console.log(`Installing ${PACKAGE_NAME}...`);
    
    const platformInfo = getPlatform();
    console.log(`Detected platform: ${platformInfo.platform}-${platformInfo.arch} (${platformInfo.target})`);
    
    const version = await getLatestVersion();
    console.log(`Latest version: ${version}`);
    
    await downloadBinary(version, platformInfo);
    
    // Verify installation
    const binPath = path.join(__dirname, 'bin', `${PACKAGE_NAME}${platformInfo.extension}`);
    if (fs.existsSync(binPath)) {
      console.log(`✅ Installation completed successfully!`);
      console.log(`🚀 Run 'npx @wadefletch/crowdcontrol --help' to get started`);
    } else {
      throw new Error('Binary not found after installation');
    }
    
  } catch (error) {
    console.error(`❌ Installation failed: ${error.message}`);
    console.error('Please check:');
    console.error('1. Your platform is supported');
    console.error('2. You have internet access');
    console.error('3. GitHub releases are available');
    console.error(`4. Visit https://github.com/${GITHUB_REPO}/releases for manual download`);
    process.exit(1);
  }
}

// Run installation
if (require.main === module) {
  install();
}