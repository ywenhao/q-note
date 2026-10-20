import { mount } from "nsh";
import { Dock } from "./Dock";
import { createState } from "./state";
mount(() => <Dock state={createState("dock")} />);
