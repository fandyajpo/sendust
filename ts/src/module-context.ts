/**
 * Module URL used to locate the bundled addon.
 *
 * The build rewrites the placeholder: the ESM output receives `import.meta.url`,
 * the CommonJS output receives a `pathToFileURL(__filename)` URL. Keeping the
 * expression out of the source lets one file compile to both formats. When the
 * source runs unbuilt (tests, examples), the loader sees the placeholder and
 * derives a base URL from the working directory instead.
 */
export const moduleUrl = "SENDUST_MODULE_URL_PLACEHOLDER";
