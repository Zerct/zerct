import type { JsonObject } from './types.ts'

function frontendPackageJson(name: string): string {
  return jsonSource({
    name,
    private: true,
    type: 'module',
    scripts: {
      typecheck: 'tsgo --noEmit',
      lint: 'oxlint src vite.config.ts --deny-warnings && fallow dead-code --production --include-dupes --include-entry-exports --fail-on-issues && fallow dupes --production --mode semantic --threshold 1 --ignore-imports --fail-on-issues && fallow health --production --max-cyclomatic 10 --max-cognitive 15 --max-crap 20 --complexity',
      build: 'vite build',
      preview: 'vite preview --host 0.0.0.0'
    },
    dependencies: {
      '@tanstack/react-router': '^1.170.8',
      react: '^19.2.6',
      'react-dom': '^19.2.6'
    },
    devDependencies: {
      '@types/node': '^25.9.1',
      '@types/react': '^19.2.15',
      '@types/react-dom': '^19.2.3',
      '@typescript/native-preview': '^7.0.0-dev.20260527.2',
      '@vitejs/plugin-react': '^6.0.2',
      fallow: '^2.84.0',
      oxlint: '^1.67.0',
      typescript: '^6.0.3',
      vite: '^8.0.14'
    }
  })
}

function frontendTsConfig(): string {
  return jsonSource({
    compilerOptions: {
      allowUnreachableCode: false,
      allowUnusedLabels: false,
      alwaysStrict: true,
      erasableSyntaxOnly: true,
      exactOptionalPropertyTypes: true,
      forceConsistentCasingInFileNames: true,
      isolatedModules: true,
      jsx: 'react-jsx',
      lib: ['ESNext', 'DOM'],
      module: 'ESNext',
      moduleDetection: 'force',
      moduleResolution: 'Bundler',
      noEmit: true,
      noFallthroughCasesInSwitch: true,
      noImplicitAny: true,
      noImplicitOverride: true,
      noImplicitReturns: true,
      noImplicitThis: true,
      noPropertyAccessFromIndexSignature: true,
      noUncheckedIndexedAccess: true,
      noUncheckedSideEffectImports: true,
      noUnusedLocals: true,
      noUnusedParameters: true,
      skipLibCheck: false,
      strict: true,
      strictBindCallApply: true,
      strictFunctionTypes: true,
      strictNullChecks: true,
      strictPropertyInitialization: true,
      target: 'ES2022',
      types: ['vite/client', 'node'],
      useUnknownInCatchVariables: true,
      verbatimModuleSyntax: true
    },
    include: ['src', 'vite.config.ts']
  })
}

function rustApiSource(): string {
  return `use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_error| "3000".to_owned());
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))?;

    for stream in listener.incoming() {
        handle(stream?)?;
    }

    Ok(())
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0_u8; 2048];
    let size = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..size]);
    let mut parts = request
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or("/");
    let origin = request
        .lines()
        .find_map(|line| line.strip_prefix("Origin: "))
        .unwrap_or("*");
    let cors_origin = allowed_origin(origin);

    if method == "OPTIONS" {
        return write_response(&mut stream, "204 No Content", "", &cors_origin);
    }

    let body = if path == "/healthz" {
        r#"{"ok":true}"#
    } else {
        r#"{"message":"hello from tovuk","backend":"rust"}"#
    };
    write_response(&mut stream, "200 OK", body, &cors_origin)
}

fn allowed_origin(request_origin: &str) -> String {
    let configured = std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_error| request_origin.to_owned());
    if configured == "*" || configured == request_origin {
        configured
    } else {
        "null".to_owned()
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    body: &str,
    origin: &str,
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\\r\\ncontent-type: application/json\\r\\ncontent-length: {}\\r\\naccess-control-allow-origin: {origin}\\r\\naccess-control-allow-methods: GET, OPTIONS\\r\\naccess-control-allow-headers: content-type, authorization\\r\\nconnection: close\\r\\n\\r\\n{body}",
        body.len()
    )
}
`
}

function frontendSource(apiBaseUrl: string): string {
  return `import { createRootRoute, createRouter, RouterProvider } from '@tanstack/react-router'
import { createRoot } from 'react-dom/client'
import './styles.css'

const apiBaseUrl = import.meta.env.VITE_API_URL ?? '${apiBaseUrl}'

function App() {
  return (
    <main>
      <section>
        <h1>Tovuk TanStack Frontend</h1>
        <p>Static runtime, dynamic Rust backend calls.</p>
        <code>{apiBaseUrl}</code>
      </section>
    </main>
  )
}

const rootRoute = createRootRoute({ component: App })
const router = createRouter({ routeTree: rootRoute })

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

const rootElement = document.getElementById('root')
if (rootElement === null) {
  throw new Error('missing root element')
}

createRoot(rootElement).render(<RouterProvider router={router} />)
`
}

function frontendViteEnvSource(): string {
  return `/// <reference types="vite/client" />

interface ViteTypeOptions {
  strictImportMetaEnv: unknown
}

interface ImportMetaEnv {
  readonly VITE_API_URL?: string
}
`
}

function jsonSource(value: JsonObject): string {
  return `${JSON.stringify(value, null, 2)}\n`
}

export { frontendPackageJson, frontendSource, frontendTsConfig, frontendViteEnvSource, rustApiSource }
