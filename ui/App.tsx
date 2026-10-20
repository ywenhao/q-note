import { mount } from "nsh";
import { Main } from "./Main";
import { createState } from "./state";
mount(() => <Main state={createState("main")} />);
