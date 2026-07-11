#!/usr/bin/env node
import { access } from 'node:fs/promises'
import { join } from 'node:path'
import { once } from 'node:events'
import { spawn } from 'node:child_process'

const FAILURE_EXIT_CODE = 1
const PROCESS_ARGUMENT_OFFSET = 2
const NEXT_ARGUMENT_OFFSET = 1
const JSON_INDENT_SPACES = 2
const binDir = import.meta.dirname

const main = async () => {
    if (!(await nativeBinaryExists())) {
        printMissingNativeBinary()
        return FAILURE_EXIT_CODE
    }
    try {
        return await runNativeBinary()
    } catch (error) {
        printLaunchError(error)
        return FAILURE_EXIT_CODE
    }
}

const nativeBinaryName = () => {
    if (process.platform === 'win32') {
        return 'tovuk-native.exe'
    }
    return 'tovuk-native'
}

const nativeBinary = join(binDir, nativeBinaryName())

const nativeBinaryExists = async () => {
    try {
        await access(nativeBinary)
        return true
    } catch {
        return false
    }
}

const runNativeBinary = async () => {
    const child = spawn(nativeBinary, process.argv.slice(PROCESS_ARGUMENT_OFFSET), {
        stdio: 'inherit',
        windowsHide: false,
    })
    const [code, signal] = await once(child, 'exit')
    if (signal) {
        process.kill(process.pid, signal)
    }
    return code ?? FAILURE_EXIT_CODE
}

const writeJsonError = (details) => {
    process.stderr.write(`${JSON.stringify(details, null, JSON_INDENT_SPACES)}\n`)
}

const printMissingNativeBinary = () => {
    if (jsonOutputRequested()) {
        writeJsonError({
            agent_instruction:
                'Reinstall with npm scripts enabled, install from GitHub Releases, Homebrew, Cargo, or rerun with TOVUK_NATIVE_BINARY pointing to a supported native binary.',
            checkout_url: null,
            code: 'native_binary_unavailable',
            docs_url: 'https://docs.tovuk.com/reference/packages',
            message: 'Tovuk native binary was not installed.',
        })
        return
    }
    process.stderr.write(
        'Tovuk native binary was not installed. Reinstall with npm scripts enabled, or install from https://github.com/tovuk/tovuk/releases.\n',
    )
}

const printLaunchError = (error) => {
    let message = String(error)
    if (error instanceof Error) {
        ;({ message } = error)
    }
    if (jsonOutputRequested()) {
        writeJsonError({
            agent_instruction:
                'Reinstall the Tovuk npm package, or install with Homebrew, Cargo, or GitHub Releases.',
            checkout_url: null,
            code: 'native_binary_launch_failed',
            docs_url: 'https://docs.tovuk.com/reference/packages',
            message: `Tovuk native binary could not start: ${message}`,
        })
        return
    }
    process.stderr.write(`Tovuk native binary could not start: ${message}\n`)
}

const jsonOutputRequested = () => {
    if (/^json$/iu.test(process.env.TOVUK_OUTPUT ?? '')) {
        return true
    }
    const args = process.argv.slice(PROCESS_ARGUMENT_OFFSET)
    for (const [index, argument] of args.entries()) {
        if (argument === '--json' || /^--output=json$/iu.test(argument)) {
            return true
        }
        if (argument === '--output' && /^json$/iu.test(args[index + NEXT_ARGUMENT_OFFSET] ?? '')) {
            return true
        }
    }
    return false
}

process.exitCode = await main()
