/** Une clases ignorando falsos. Suficiente para lo que hace esta app. */
export function cn(...clases: Array<string | false | null | undefined>): string {
  return clases.filter(Boolean).join(" ");
}
