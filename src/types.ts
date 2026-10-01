export interface Widget {
  id: string;
  name: string;
  monitor: string | null;
  x: number;
  y: number;
  w: number;
  h: number;
  html: string;
  css: string;
  js: string;
  opacity: number;
  enabled: boolean;
}

export interface Config {
  widgets: Widget[];
}
