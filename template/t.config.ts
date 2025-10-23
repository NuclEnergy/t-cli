import { AllLanguages, extract_languages_from_config } from "@nuclenergy/t";
import { TConfig } from "@nuclenergy/t/tconfig";

const config = {
  languages: {
    name: "en",
    children: [
      {
        name: "zh",
      },
    ],
  },
  targets: [
    {
      includes: ["/src"],
      excludes: ["node_modules", ".*"],
      output: "_t",
      fnNames: ["t"],
    },
  ],
} as const satisfies TConfig;

export type Language = AllLanguages<typeof config>;

export const languages = extract_languages_from_config(config);
export const defaultLanguage = config.languages.name;
export const langKey = "lang";

export default config;
