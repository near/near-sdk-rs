//@nearfile
// Exporting a new class TextMessage so it can be used outside of this file.
export class TextMessage {
  sender: string;
  text: string;
  number: u64;
  isRead: bool;
}
