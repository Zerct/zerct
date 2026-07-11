import { copyFile, lstat, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises'
import { platform, tmpdir } from 'node:os'
import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import installer from '../install-policy.mjs'
import { join } from 'node:path'
import { promisify } from 'node:util'
import test from 'node:test'

const execFileAsync = promisify(execFile)
const sourceRoot = join(import.meta.dirname, '..')
const FIXTURE_FILES = [
    'install-policy.mjs',
    'install.mjs',
    'native-release-targets.json',
    'package.json',
]
const HELLO_SHA256 = '2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824'
const FIRST_REQUEST_COUNT = 1
const INITIAL_REDIRECT_COUNT = 0
const TEST_MAX_BYTES = 5
const SHA256_HEX_LENGTH = 64
const {
    assertAllowedDownloadUrl,
    assertDownloadSize,
    fetchResponse,
    parseChecksum,
    verifySha256,
    writeResponse,
} = installer

const untrustedFetch = () =>
    new Response(null, { headers: { location: 'https://example.com/native' }, status: 302 })

const loopingFetch = () =>
    new Response(null, { headers: { location: 'https://github.com/again' }, status: 302 })

const assertInvalidDeclaredSizes = () => {
    for (const contentLength of ['invalid', '0', '6']) {
        const response = new Response('small', { headers: { 'content-length': contentLength } })
        assert.throws(() => assertDownloadSize(response, TEST_MAX_BYTES), /native binary/u)
    }
}

const assertStreamedSizes = async (fixture) => {
    const emptyResponse = new Response(new Uint8Array())
    const emptyPath = join(fixture, 'empty')
    await assert.rejects(
        writeResponse(emptyResponse, emptyPath, TEST_MAX_BYTES),
        /response is empty/u,
    )
    const largeResponse = new Response('larger')
    const largePath = join(fixture, 'large')
    await assert.rejects(writeResponse(largeResponse, largePath, TEST_MAX_BYTES), /exceeds/u)
    const exactPath = join(fixture, 'exact')
    await writeResponse(new Response('small'), exactPath, TEST_MAX_BYTES)
    assert.equal(await readFile(exactPath, 'utf8'), 'small')
}

const createFixture = async (context) => {
    const fixture = await mkdtemp(join(process.env.RUNNER_TEMP ?? tmpdir(), 'tovuk-npm-test-'))
    context.after(() => rm(fixture, { force: true, recursive: true }))
    await mkdir(join(fixture, 'bin'))
    await copyFile(join(sourceRoot, 'bin', 'tovuk.mjs'), join(fixture, 'bin', 'tovuk.mjs'))
    await Promise.all(
        FIXTURE_FILES.map((file) => copyFile(join(sourceRoot, file), join(fixture, file))),
    )
    return fixture
}

const runInstaller = (fixture, nativeBinary) =>
    execFileAsync(process.execPath, [join(fixture, 'install.mjs')], {
        env: { ...process.env, TOVUK_NATIVE_BINARY: nativeBinary },
    })

const nativeTestExecutable = () => {
    if (platform() === 'win32') {
        return join(process.env.SystemRoot ?? String.raw`C:\Windows`, 'System32', 'cmd.exe')
    }
    return '/usr/bin/true'
}

const nativeTestArguments = () => {
    if (platform() === 'win32') {
        return ['/c', 'exit', '0']
    }
    return []
}

test('installs and launches an explicitly supplied regular executable', async (context) => {
    const fixture = await createFixture(context)
    await runInstaller(fixture, nativeTestExecutable())
    const { stdout } = await execFileAsync(process.execPath, [
        join(fixture, 'bin', 'tovuk.mjs'),
        ...nativeTestArguments(),
    ])
    assert.equal(stdout, '')
})

test('rejects a local source that is not a regular file', async (context) => {
    const fixture = await createFixture(context)
    await assert.rejects(runInstaller(fixture, fixture), /not a regular file/u)
})

test(
    'replaces an existing native-binary symlink without writing through it',
    { skip: platform() === 'win32' },
    async (context) => {
        const fixture = await createFixture(context)
        const victim = join(fixture, 'victim')
        const destination = join(fixture, 'bin', 'tovuk-native')
        await writeFile(victim, 'unchanged\n')
        await symlink(victim, destination)

        await runInstaller(fixture, nativeTestExecutable())

        const destinationMetadata = await lstat(destination)
        assert.equal(destinationMetadata.isSymbolicLink(), false)
        assert.equal(await readFile(victim, 'utf8'), 'unchanged\n')
    },
)

test('reports the stable JSON error contract when the native binary is absent', async (context) => {
    const fixture = await createFixture(context)
    await assert.rejects(
        execFileAsync(process.execPath, [join(fixture, 'bin', 'tovuk.mjs'), '--json']),
        (error) => {
            const details = JSON.parse(error.stderr)
            assert.deepEqual(details, {
                agent_instruction:
                    'Reinstall with npm scripts enabled, install from GitHub Releases, Homebrew, Cargo, or rerun with TOVUK_NATIVE_BINARY pointing to a supported native binary.',
                checkout_url: null,
                code: 'native_binary_unavailable',
                docs_url: 'https://docs.tovuk.com/reference/packages',
                message: 'Tovuk native binary was not installed.',
            })
            return true
        },
    )
})

test('accepts only credential-free HTTPS release hosts', () => {
    for (const value of [
        'https://github.com/tovuk/tovuk/releases/download/v1/asset',
        'https://objects.githubusercontent.com/asset',
        'https://release-assets.githubusercontent.com/asset',
    ]) {
        assert.doesNotThrow(() => assertAllowedDownloadUrl(value))
    }
    for (const value of [
        'http://github.com/tovuk/tovuk/releases/download/v1/asset',
        'https://example.com/asset',
        'https://user:secret@github.com/asset',
        'https://github.com:8443/asset',
    ]) {
        assert.throws(() => assertAllowedDownloadUrl(value), /refusing untrusted download URL/u)
    }
})

test('validates every redirect and caps the redirect chain', async () => {
    const requests = []
    const trustedFetch = (url) => {
        requests.push(url)
        if (requests.length === FIRST_REQUEST_COUNT) {
            return new Response(null, {
                headers: { location: 'https://objects.githubusercontent.com/native' },
                status: 302,
            })
        }
        return new Response('native', { status: 200 })
    }
    const response = await fetchResponse(
        'https://github.com/release',
        INITIAL_REDIRECT_COUNT,
        trustedFetch,
    )
    assert.equal(await response.text(), 'native')
    assert.deepEqual(requests, [
        'https://github.com/release',
        'https://objects.githubusercontent.com/native',
    ])

    await assert.rejects(
        fetchResponse('https://github.com/release', INITIAL_REDIRECT_COUNT, untrustedFetch),
        /refusing untrusted download URL/u,
    )

    await assert.rejects(
        fetchResponse('https://github.com/release', INITIAL_REDIRECT_COUNT, loopingFetch),
        /too many redirects/u,
    )
})

test('rejects invalid declared and streamed native-binary sizes', async (context) => {
    assertInvalidDeclaredSizes()
    const fixture = await createFixture(context)
    await assertStreamedSizes(fixture)
})

test('binds a single checksum line to the requested asset', () => {
    assert.equal(parseChecksum(`${HELLO_SHA256}  tovuk-native\n`, 'tovuk-native'), HELLO_SHA256)
    assert.equal(parseChecksum(`${HELLO_SHA256}\n`, 'tovuk-native'), HELLO_SHA256)
    assert.throws(
        () => parseChecksum(`${HELLO_SHA256}  different\n`, 'tovuk-native'),
        /names different/u,
    )
    assert.throws(
        () => parseChecksum(`${HELLO_SHA256}\n${HELLO_SHA256}\n`, 'tovuk-native'),
        /exactly one/u,
    )
    assert.throws(() => parseChecksum('not-a-digest\n', 'tovuk-native'), /does not contain/u)
})

test('streams the local checksum and rejects a mismatch', async (context) => {
    const fixture = await createFixture(context)
    const path = join(fixture, 'payload')
    await writeFile(path, 'hello')
    await verifySha256(path, HELLO_SHA256)
    await assert.rejects(verifySha256(path, '0'.repeat(SHA256_HEX_LENGTH)), /checksum mismatch/u)
})
