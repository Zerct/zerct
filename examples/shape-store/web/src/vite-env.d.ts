/// <reference types="vite/client" />

interface ViteTypeOptions {
  strictImportMetaEnv: unknown
}

interface ImportMetaEnv {
  readonly VITE_API_URL?: string
  readonly VITE_PRODUCT_MEDIA_BASE_URL?: string
}
