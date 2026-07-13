import { create } from 'zustand'

export type Lang = 'en' | 'de'

const STORAGE_KEY = 'loglens_lang'

interface Dict {
  [key: string]: string | Dict
}

const translations: Record<Lang, Dict> = {
  en: {
    nav: {
      logs: 'Logs', clusters: 'Clusters', timeline: 'Timeline',
      sources: 'Sources', settings: 'Settings',
    },
    logs: {
      searchPlaceholder: 'Search logs...',
      entries: 'entries',
      entryDetail: 'Entry Detail',
      timestamp: 'Timestamp', level: 'Level', service: 'Service',
      source: 'Source', message: 'Message', stacktrace: 'Stacktrace',
      aiAnalyzing: 'AI analyzing...',
      aiAnalysis: 'AI Analysis', confidence: 'confidence',
      what: 'What:', why: 'Why:', fixSuggestions: 'Fix suggestions',
    },
    clusters: {
      errorClusters: 'Error Clusters',
      rootCauseAnalysis: 'Root Cause Analysis',
      template: 'Template', count: 'Count', level: 'Level',
      analyzingAi: 'Analyzing with AI...',
      rootCause: 'Root Cause', contributingFactors: 'Contributing Factors',
      fixSteps: 'Fix Steps', confidence: 'Confidence',
    },
    timeline: {
      title: 'Log Timeline',
      errorsWarnings: 'Errors & Warnings per 5 min',
    },
    sources: {
      title: 'Log Sources',
      addFileTitle: 'Add File / Directory',
      labelOptional: 'Label (optional)',
      watch: 'Watch',
      addDockerTitle: 'Add Docker Container',
      fileSourceAdded: 'File source added',
      dockerSourceAdded: 'Docker source added',
      activeSources: 'Active Sources',
      noSources: 'No sources configured.',
      remove: 'Remove',
      parser: 'Parser:', parserAuto: 'Auto-detect',
    },
    settings: {
      title: 'Settings',
      aiBackend: 'AI Backend',
      apiKeySet: 'Set', apiKeyNotConfigured: 'Not configured',
      save: 'Save',
      testing: 'Testing...', testConnection: 'Test connection',
      backendReachable: 'AI backend reachable',
      backendUnreachable: 'AI backend not reachable',
      general: 'General',
      autoCluster: 'Auto-cluster similar errors',
      maxEntries: 'Max entries in memory',
      saveSettings: 'Save Settings',
      settingsSaved: 'Settings saved',
      apiKeySaved: 'API key saved',
      customParsers: 'Custom Parsers',
      customParsersHint: 'Define a named parser via a regex with named capture groups (timestamp, level, service, message, all optional) for log formats none of the built-in parsers recognize. Assign one to a source in Log Sources.',
      parserName: 'Parser name', timestampFormatOptional: 'Timestamp format (optional, e.g. %Y/%m/%d %H:%M:%S)',
      addParser: 'Add Parser',
    },
  },
  de: {
    nav: {
      logs: 'Logs', clusters: 'Cluster', timeline: 'Verlauf',
      sources: 'Quellen', settings: 'Einstellungen',
    },
    logs: {
      searchPlaceholder: 'Logs durchsuchen...',
      entries: 'Einträge',
      entryDetail: 'Eintrag-Detail',
      timestamp: 'Zeitstempel', level: 'Level', service: 'Service',
      source: 'Quelle', message: 'Nachricht', stacktrace: 'Stacktrace',
      aiAnalyzing: 'KI analysiert...',
      aiAnalysis: 'KI-Analyse', confidence: 'Konfidenz',
      what: 'Was:', why: 'Warum:', fixSuggestions: 'Lösungsvorschläge',
    },
    clusters: {
      errorClusters: 'Fehler-Cluster',
      rootCauseAnalysis: 'Root-Cause-Analyse',
      template: 'Vorlage', count: 'Anzahl', level: 'Level',
      analyzingAi: 'KI analysiert...',
      rootCause: 'Ursache', contributingFactors: 'Einflussfaktoren',
      fixSteps: 'Lösungsschritte', confidence: 'Konfidenz',
    },
    timeline: {
      title: 'Log-Verlauf',
      errorsWarnings: 'Fehler & Warnungen pro 5 Min.',
    },
    sources: {
      title: 'Log-Quellen',
      addFileTitle: 'Datei / Verzeichnis hinzufügen',
      labelOptional: 'Bezeichnung (optional)',
      watch: 'Beobachten',
      addDockerTitle: 'Docker-Container hinzufügen',
      fileSourceAdded: 'Datei-Quelle hinzugefügt',
      dockerSourceAdded: 'Docker-Quelle hinzugefügt',
      activeSources: 'Aktive Quellen',
      noSources: 'Keine Quellen konfiguriert.',
      remove: 'Entfernen',
      parser: 'Parser:', parserAuto: 'Automatisch erkennen',
    },
    settings: {
      title: 'Einstellungen',
      aiBackend: 'KI-Backend',
      apiKeySet: 'Gesetzt', apiKeyNotConfigured: 'Nicht konfiguriert',
      save: 'Speichern',
      testing: 'Teste...', testConnection: 'Verbindung testen',
      backendReachable: 'KI-Backend erreichbar',
      backendUnreachable: 'KI-Backend nicht erreichbar',
      general: 'Allgemein',
      autoCluster: 'Ähnliche Fehler automatisch clustern',
      maxEntries: 'Max. Einträge im Speicher',
      saveSettings: 'Einstellungen speichern',
      settingsSaved: 'Einstellungen gespeichert',
      apiKeySaved: 'API-Key gespeichert',
      customParsers: 'Eigene Parser',
      customParsersHint: 'Definiere einen benannten Parser über eine Regex mit benannten Capture-Groups (timestamp, level, service, message, alle optional) für Log-Formate, die keiner der eingebauten Parser erkennt. Einer Quelle unter Log-Quellen zuweisen.',
      parserName: 'Parser-Name', timestampFormatOptional: 'Zeitstempel-Format (optional, z. B. %Y/%m/%d %H:%M:%S)',
      addParser: 'Parser hinzufügen',
    },
  },
}

interface LangState {
  lang: Lang
  setLang: (lang: Lang) => void
  toggle: () => void
}

export const useLangStore = create<LangState>((set) => ({
  lang: (localStorage.getItem(STORAGE_KEY) as Lang) || 'en',
  setLang: (lang) => {
    localStorage.setItem(STORAGE_KEY, lang)
    set({ lang })
  },
  toggle: () => set((s) => {
    const next: Lang = s.lang === 'en' ? 'de' : 'en'
    localStorage.setItem(STORAGE_KEY, next)
    return { lang: next }
  }),
}))

export function getLang(): Lang {
  return useLangStore.getState().lang
}

function resolve(dict: Dict, path: string): string {
  const parts = path.split('.')
  let node: string | Dict | undefined = dict
  for (const p of parts) {
    node = typeof node === 'object' ? node[p] : undefined
  }
  return typeof node === 'string' ? node : path
}

export function t(path: string): string {
  return resolve(translations[getLang()], path)
}

export function useT() {
  const lang = useLangStore((s) => s.lang)
  return (path: string) => resolve(translations[lang], path)
}

export function dateLocale(): string {
  return getLang() === 'de' ? 'de-CH' : 'en-US'
}
