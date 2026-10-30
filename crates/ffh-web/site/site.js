// The static site's own settings (docs/WEB.md, "The classic site"): tools/web-site.sh copies this
// file to web/site.js, or writes it with the name given by `--saves NAME`.

/** The IndexedDB database the site keeps its saves in. Two sites on one origin (the main site and
 *  classic/ below it on GitHub Pages) need different ones, or each would list, load and overwrite
 *  the other's games. */
export const SAVES_DB = "ffh-play";
