/**
 * Module URL used to locate the bundled addon.
 *
 * The build rewrites the placeholder: the ESM output receives `import.meta.url`,
 * the CommonJS output receives a `pathToFileURL(__filename)` URL. Keeping the
 * expression out of the source lets one file compile to both formats.
 */
export const moduleUrl = "SENDUST_MODULE_URL_PLACEHOLDER";
