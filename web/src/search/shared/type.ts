type I18nKey = string; // just for naming
type Url = string; // just for naming

export type ModuleSymbol = {
  module_path: string;
  symbol_name: string;
};
export type TracePath = ModuleSymbol[];

// Trace result might have multiple i18n keys, each i18n key might be
// used in multiple module symbols, and each module symbol might be
// used in multiple urls through different paths.
export type TraceResult = Record<
  I18nKey,
  Record<Url, Record<string, TracePath[]>>
>;
