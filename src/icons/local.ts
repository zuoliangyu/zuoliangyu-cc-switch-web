import eflowcodeLogo from "@/assets/icons/eflowcode.png";

const localIcons: Record<string, string> = {
  eflowcode: eflowcodeLogo,
};

export const localIconList = Object.keys(localIcons);

export function hasLocalIcon(name: string): boolean {
  return name.toLowerCase() in localIcons;
}

export function getLocalIconUrl(name: string): string {
  return localIcons[name.toLowerCase()] || "";
}
