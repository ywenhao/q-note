import { mount } from "nsh";
import { Editor } from "./Editor";
import { createState } from "./state";
mount(() => <Editor state={createState("editor")} />);
