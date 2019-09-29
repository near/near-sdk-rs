//@nearfile
// Basic data model
export class Greeter {
  text: string;

  constructor(text: string) {
    this.text = text;
  }

  greet(userId: string): string {
    return "Hello, " + userId;
  }
}
