export type WidgetType =
  | 'action_button'
  | 'vertical_fader'
  | 'media_bar'
  | 'rotary_dial'
  | 'system_gauge'
  | 'rocker_toggle'
  | 'live_chat'
  | 'video_feed'
  | 'stream_notes';

export interface WidgetManifest {
  type: WidgetType;
  name: string;
  category: 'Actions' | 'Audio & Media' | 'Monitoring' | 'Live Streaming' | 'Utilities';
  description: string;
  icon: string;
  defaultSpan: { cols: number; rows: number };
  minSpan: { cols: number; rows: number };
  maxSpan: { cols: number; rows: number };
  defaultConfig: Record<string, any>;
}

export interface DeckWidgetItem {
  id: string;
  type: WidgetType;
  title?: string;
  colSpan: number;
  rowSpan: number;
  config: Record<string, any>;
  state?: Record<string, any>;
}

export interface DeckGridConfig {
  rows: number;
  cols: number;
  gap: number;
  borderRadius: number;
  backgroundTheme: string;
}
