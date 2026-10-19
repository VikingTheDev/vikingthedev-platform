declare namespace App {
  interface Locals {
    user: import("@vikingthedev/session").SessionUser | null;
  }
}
