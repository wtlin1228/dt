import Kirby, { Power, Pink as KirbyPink, Puffy } from "./kirby";
import * as Hawk from "./hawk";
const sugar = "",
  salt = "";
const cruet = [sugar, salt];
export class PicnicBox {
  constructor() {
    this.cruet = cruet;
    this.sandwich = "beef sandwich";
    this.cookie = { color: KirbyPink, texture: Puffy };
  }
}
const deliverPicnicBox = (location) => {
  Kirby.bring(new PicnicBox());
  Kirby.goto(location);
  Kirby.put();
};
function WelcomeMessage() {
  return "Welcome 🤗 Kirby is delivering your picnic box 👜";
}
export { WelcomeMessage as welcome };
export function InvitationCard() {
  const [opened, setOpened] = React.useState(false);
  if (!opened) {
    return (
      <Hawk.PigNose
        onPush={() => {
          setOpened(true);
          deliverPicnicBox();
        }}
      />
    );
  } else {
    return <WelcomeMessage />;
  }
}
export default InvitationCard;
export * from "./wild";
export * as Wild from "./wild";
