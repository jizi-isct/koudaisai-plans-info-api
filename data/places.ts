export type VenueType = "building" | "stage" | "outdoor" | "food_stall_area";

export interface District {
  id: string;
  name: string;
}

export interface Venue {
  id: string;
  district_id: string;
  name: string;
  type: VenueType;
}

/** 建物内の部屋 */
export interface BuildingRoom {
  id: string;
  venue_id: string;
  name: string;
  /** 階数。負数は地下を表す（-1 = B1F） */
  floor: number;
}

/** 模擬店の区画 */
export interface FoodStallSlot {
  id: string;
  venue_id: string;
  name: string;
}

export type Room = BuildingRoom | FoodStallSlot;

export interface PlacesData {
  districts: District[];
  venues: Venue[];
  rooms: Room[];
}