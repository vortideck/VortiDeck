
  export type Background =
  | { type: "css"; value: string } // CSS string for solid colors, gradients, etc.
  | { type: "file"; value: string } // Path to an image or video file
  | { type: "particles"; config: Record<string, any> }; // Particle effects with configuration


export type ButtonMode = string | null
export type ButtonIconFit = "cover" | "contain"

export type Button = {
  id: number; // Unique identifier for the button
  deck_id: number;
  label: string; // Button label or name
  icon: string | null; // Optional icon path or identifier
  background: string | null;
  action: string | null; // Action triggered by the button
  color?: string; // Optional color for the button
  position: number;
  mode: ButtonMode
  execution_mode: string | null;
  execution_config: string | null;
  icon_updated_at: number | null;
  icon_fit: ButtonIconFit;
};

export type DeckSettings = {
  id: number;
  button_spacing: number;
  grid_margin: number;
  group_buttons: boolean;
  maximize_button_space: boolean;
  grid: string;
  deck_id: number;
  buttons_padding: number;
  sides_enabled: boolean;
  orientation_sync_enabled: boolean;
  active_theme: number | null;
};

export type Deck = {
  id: number; // Unique identifier for the deck
  name: string; // Name of the deck
  game?: string; // Optional auto-activate game identifier
  theme?: any; // Associated theme
  created_at?: string; // Optional creation timestamp
  layout: {
    grid: {
      portrait: [number, number]; // Tuple for portrait layout [rows, columns]
      landscape: [number, number]; // Tuple for landscape layout [rows, columns]
    };
    background?: Background; // Flexible background configuration
  };
  buttons: Button[]; // Array of buttons on the deck
  settings: DeckSettings;
};

