/** Audible stores Find details can search; mirrors `REGIONS` in `metadata/audible.rs`. */
export const AUDIBLE_REGIONS = [
    { code: "us", label: "United States" },
    { code: "uk", label: "United Kingdom" },
    { code: "ca", label: "Canada" },
    { code: "au", label: "Australia" },
    { code: "de", label: "Germany" },
    { code: "fr", label: "France" },
    { code: "it", label: "Italy" },
    { code: "es", label: "Spain" },
    { code: "jp", label: "Japan" },
    { code: "in", label: "India" },
] as const;

export const DEFAULT_AUDIBLE_REGION = "us";
