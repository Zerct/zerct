#!/usr/bin/env node
import { arch, platform } from 'node:os'
import { basename, dirname, join } from 'node:path'
import { chmod, copyFile, mkdir, mkdtemp, readFile, rename, rm, stat } from 'node:fs/promises'
import installPolicy from './install-policy.mjs'

const FILE_MODE = 0o755
const EMPTY_SIZE = 0
const { download, fetchText, parseChecksum, verifySha256 } = installPolicy
const packageRoot = import.meta.dirname
const manifest = JSON.parse(await readFile(join(packageRoot, 'package.json'), 'utf8'))
const nativeTargets = JSON.parse(
    await readFile(join(packageRoot, 'native-release-targets.json'), 'utf8'),
).targets

const main = async () => {
    await mkdir(dirname(binaryPath), { recursive: true })
    const localBinary = process.env.TOVUK_NATIVE_BINARY
    if (localBinary) {
        await installFromLocal(localBinary)
        return
    }
    await installFromRelease()
}

const installFromLocal = async (source) => {
    await assertRegularFile(source)
    const { tempDir, tempPath } = await temporaryDownload(nativeBinaryName())
    try {
        await copyFile(source, tempPath)
        await activateDownload(tempPath)
    } finally {
        await rm(tempDir, { force: true, recursive: true })
    }
}

const releaseDownload = () => {
    const target = nativeTarget()
    const asset = `tovuk-${manifest.version}-${target.triple}${target.asset_ext}`
    const url = `https://github.com/tovuk/tovuk/releases/download/v${manifest.version}/${asset}`
    return { asset, checksumUrl: `${url}.sha256`, url }
}

const temporaryDownload = async (asset) => {
    const tempDir = await mkdtemp(join(dirname(binaryPath), '.tovuk-install-'))
    return { tempDir, tempPath: join(tempDir, basename(asset)) }
}

const activateDownload = async (tempPath) => {
    await chmod(tempPath, FILE_MODE)
    await rename(tempPath, binaryPath)
}

const promoteDownload = async (tempPath, checksumUrl, asset) => {
    await assertRegularFile(tempPath)
    const expectedSha256 = parseChecksum(await fetchText(checksumUrl), asset)
    await verifySha256(tempPath, expectedSha256)
    await activateDownload(tempPath)
}

const installFromRelease = async () => {
    const { asset, checksumUrl, url } = releaseDownload()
    const { tempDir, tempPath } = await temporaryDownload(asset)
    try {
        await download(url, tempPath)
        await promoteDownload(tempPath, checksumUrl, asset)
    } catch (error) {
        throw new Error(
            `Could not install native Tovuk binary from ${url}: ${errorMessage(error)}`,
            { cause: error },
        )
    } finally {
        await rm(tempDir, { force: true, recursive: true })
    }
}

const nativeTarget = () => {
    const operatingSystem = platform()
    const processor = arch()
    const target = nativeTargets.find(
        (item) => item.node.arch === processor && item.node.platform === operatingSystem,
    )
    if (target?.libc === 'glibc' && linuxLibc() !== 'glibc') {
        throw new Error(
            `Unsupported Tovuk native target: ${operatingSystem}/${processor} requires glibc Linux. Alpine/musl Linux is not supported by the published native binaries yet.`,
        )
    }
    if (target) {
        return target
    }
    throw new Error(`Unsupported Tovuk native target: ${operatingSystem}/${processor}`)
}

const nativeBinaryName = () => {
    if (platform() === 'win32') {
        return 'tovuk-native.exe'
    }
    return 'tovuk-native'
}

const binaryPath = join(packageRoot, 'bin', nativeBinaryName())

const linuxLibc = () => {
    if (platform() !== 'linux') {
        return ''
    }
    const glibcVersion = process.report?.getReport?.()?.header?.glibcVersionRuntime
    if (typeof glibcVersion === 'string' && glibcVersion.length > EMPTY_SIZE) {
        return 'glibc'
    }
    return 'musl'
}

const errorMessage = (error) => {
    if (error instanceof Error) {
        return error.message
    }
    return String(error)
}

const assertRegularFile = async (path) => {
    const metadata = await stat(path)
    if (!metadata.isFile()) {
        throw new Error('downloaded native binary is not a regular file')
    }
}

await main()
