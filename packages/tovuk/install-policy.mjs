import { basename } from 'node:path'
import { createHash } from 'node:crypto'
import { createReadStream } from 'node:fs'
import { open } from 'node:fs/promises'

const REQUEST_TIMEOUT_MS = 30_000
const MAX_REDIRECTS = 5
const KIBIBYTE_BYTES = 1024
const HASH_BLOCK_KIBIBYTES = 64
const MAX_BINARY_MEBIBYTES = 100
const MAX_BINARY_BYTES = MAX_BINARY_MEBIBYTES * KIBIBYTE_BYTES * KIBIBYTE_BYTES
const MAX_CHECKSUM_BYTES = 4096
const HASH_BUFFER_BYTES = HASH_BLOCK_KIBIBYTES * KIBIBYTE_BYTES
const FILE_MODE = 0o755
const HTTP_SUCCESS = 200
const HTTP_REDIRECT_MIN = 300
const HTTP_REDIRECT_MAX = 400
const NEXT_REDIRECT = 1
const EXPECTED_CHECKSUM_LINES = 1
const EMPTY_SIZE = 0
const ALLOWED_DOWNLOAD_HOSTS = new Set([
    'github.com',
    'objects.githubusercontent.com',
    'release-assets.githubusercontent.com',
])

const assertDownloadSize = (response, maximumBytes = MAX_BINARY_BYTES) => {
    const rawContentLength = response.headers.get('content-length')
    if (rawContentLength === null) {
        return
    }
    const contentLength = Number(rawContentLength)
    if (!Number.isSafeInteger(contentLength) || contentLength <= EMPTY_SIZE || contentLength > maximumBytes) {
        throw new Error(`native binary must contain between 1 and ${maximumBytes} bytes`)
    }
}

const writeResponse = async (response, destination, maximumBytes = MAX_BINARY_BYTES) => {
    const body = responseBody(response)
    const file = await open(destination, 'wx', FILE_MODE)
    try {
        const downloadedBytes = await writeResponseBody(body, file, maximumBytes)
        assertNonemptyDownload(downloadedBytes)
    } finally {
        await file.close()
    }
}

const writeResponseBody = async (body, file, maximumBytes) => {
    let downloadedBytes = EMPTY_SIZE
    for await (const chunk of body) {
        downloadedBytes += chunk.length
        if (downloadedBytes > maximumBytes) {
            throw new Error(`native binary exceeds the ${maximumBytes}-byte download limit`)
        }
        await file.writeFile(chunk)
    }
    return downloadedBytes
}

const assertNonemptyDownload = (downloadedBytes) => {
    if (downloadedBytes === EMPTY_SIZE) {
        throw new Error('native binary response is empty')
    }
}

const responseBody = (response) => {
    if (!response.body) {
        throw new Error('native binary response has no body')
    }
    return response.body
}

const download = async (url, destination) => {
    const response = await fetchResponse(url)
    assertDownloadSize(response)
    await writeResponse(response, destination)
}

const fetchResponse = async (url, redirects = EMPTY_SIZE, fetchImplementation = fetch) => {
    assertAllowedDownloadUrl(url)
    const response = await fetchImplementation(url, {
        redirect: 'manual',
        signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    })
    const location = response.headers.get('location')
    if (response.status >= HTTP_REDIRECT_MIN && response.status < HTTP_REDIRECT_MAX && location) {
        return followRedirect(response, { location, redirects, url }, fetchImplementation)
    }
    await assertSuccessfulResponse(response)
    return response
}

const followRedirect = async (response, redirect, fetchImplementation) => {
    await response.body?.cancel()
    if (redirect.redirects >= MAX_REDIRECTS) {
        throw new Error(`too many redirects while downloading ${redirect.url}`)
    }
    return fetchResponse(
        new URL(redirect.location, redirect.url).toString(),
        redirect.redirects + NEXT_REDIRECT,
        fetchImplementation,
    )
}

const assertSuccessfulResponse = async (response) => {
    if (response.status === HTTP_SUCCESS) {
        return
    }
    await response.body?.cancel()
    throw new Error(`HTTP ${response.status}`)
}

const fetchText = async (url) => {
    const response = await fetchResponse(url)
    return readChecksumBody(responseBody(response))
}

const readChecksumBody = async (body) => {
    const chunks = []
    let size = EMPTY_SIZE
    for await (const chunk of body) {
        size += chunk.length
        if (size > MAX_CHECKSUM_BYTES) {
            throw new Error('checksum response is too large')
        }
        chunks.push(chunk)
    }
    return Buffer.concat(chunks).toString('utf8')
}

const parseChecksum = (text, asset) => {
    const line = checksumLine(text, asset)
    const [digest, ...nameParts] = line.split(/\s+/u)
    assertChecksumDigest(digest, asset)
    assertChecksumAsset(nameParts, asset)
    return digest.toLowerCase()
}

const checksumLine = (text, asset) => {
    const lines = text
        .split(/\r?\n/u)
        .map((item) => item.trim())
        .filter(Boolean)
    if (lines.length === EMPTY_SIZE) {
        throw new Error(`checksum file for ${asset} is empty`)
    }
    if (lines.length !== EXPECTED_CHECKSUM_LINES) {
        throw new Error(`checksum file for ${asset} must contain exactly one non-empty line`)
    }
    return lines.at(EMPTY_SIZE)
}

const assertChecksumDigest = (digest, asset) => {
    if (!/^[a-fA-F0-9]{64}$/u.test(digest)) {
        throw new Error(`checksum file for ${asset} does not contain a SHA-256 digest`)
    }
}

const assertChecksumAsset = (nameParts, asset) => {
    if (nameParts.length === EMPTY_SIZE) {
        return
    }
    const listedAsset = basename(nameParts.join(' ').replace(/^\*/u, ''))
    if (listedAsset !== asset) {
        throw new Error(`checksum file names ${listedAsset}, expected ${asset}`)
    }
}

const verifySha256 = async (path, expectedSha256) => {
    const actualDigest = await sha256File(path)
    if (actualDigest !== expectedSha256) {
        throw new Error(`native binary checksum mismatch: expected ${expectedSha256}, got ${actualDigest}`)
    }
}

const sha256File = async (path) => {
    const digest = createHash('sha256')
    for await (const chunk of createReadStream(path, { highWaterMark: HASH_BUFFER_BYTES })) {
        digest.update(chunk)
    }
    return digest.digest('hex')
}

const assertAllowedDownloadUrl = (value) => {
    const url = new URL(value)
    const trusted =
        url.protocol === 'https:' &&
        !url.username &&
        !url.password &&
        !url.port &&
        ALLOWED_DOWNLOAD_HOSTS.has(url.hostname)
    if (!trusted) {
        throw new Error(`refusing untrusted download URL: ${url.origin}`)
    }
}

export default Object.freeze({
    assertAllowedDownloadUrl,
    assertDownloadSize,
    download,
    fetchResponse,
    fetchText,
    parseChecksum,
    verifySha256,
    writeResponse,
})
