import { Text, View } from "nsh";
import { Button } from "nsh/base";
import { currentWindow, isAvailable } from "nsh/desktop";
import type { NoteModel } from "../model";
import { colors as C } from "../theme";
import { Icon, QMark } from "../ui";

export function WindowBar(props: {
  model: NoteModel;
  title: string;
  editor?: boolean;
}) {
  const m = props.model;
  const window = isAvailable ? currentWindow() : null;
  return (
    <View
      style={{
        flexDirection: "row",
        height: props.editor ? 28 : 24,
        alignItems: "center",
        paddingBottom: props.editor ? 4 : 0,
        marginBottom: 4,
        gap: 7,
      }}
    >
      <View
        desktop={{ windowDrag: true }}
        style={{
          flex: 1,
          flexDirection: "row",
          alignItems: "center",
          gap: 7,
          height: "100%",
          minWidth: 0,
        }}
      >
        <If when={!props.editor}>
          <QMark size={22} />
        </If>
        <Text
          style={{
            numberOfLines: 1,
            fontSize: 15.12,
            fontWeight: "600",
            color: C.foreground,
          }}
        >
          {props.title}
        </Text>
      </View>
      <View
        style={{
          flexDirection: "row",
          gap: 2,
          padding: 2,
          borderWidth: 1,
          borderColor: "#ffffff6b",
          borderRadius: 12,
          backgroundColor: "#ffffff42",
          boxShadow: "inset 0 1px 0 #ffffff70, 0 4px 12px #1f23280f",
        }}
      >
        <Button
          accessibilityLabel={
            m.settings().alwaysOnTop ? m.t().alwaysOff : m.t().alwaysOn
          }
          onPress={() => {
            void m.changeSettings({ alwaysOnTop: !m.settings().alwaysOnTop });
          }}
          testID="window-pin"
          style={{
            width: 18,
            height: 18,
            borderRadius: 9,
            alignItems: "center",
            justifyContent: "center",
          }}
          hoverStyle={{ backgroundColor: "#007aff33" }}
        >
          <Icon
            name="pin"
            size={10}
            color={m.settings().alwaysOnTop ? C.primary : "#3c3c43b8"}
          />
        </Button>
        <Button
          accessibilityLabel={m.t().minimize}
          onPress={() => window?.minimize()}
          testID="window-minimize"
          style={{
            width: 18,
            height: 18,
            borderRadius: 9,
            alignItems: "center",
            justifyContent: "center",
          }}
          hoverStyle={{ backgroundColor: "#ffcc00" }}
        >
          <Icon name="minus" size={10} color="#3c3c43b8" />
        </Button>
        <Button
          accessibilityLabel={m.t().closePanel}
          onPress={() => {
            if (props.editor) void m.closeEditor();
            else window?.hide();
          }}
          testID="window-close"
          style={{
            width: 18,
            height: 18,
            borderRadius: 9,
            alignItems: "center",
            justifyContent: "center",
          }}
          hoverStyle={{ backgroundColor: "#ff3b3066" }}
        >
          <Icon name="close" size={10} color="#3c3c43b8" />
        </Button>
      </View>
    </View>
  );
}
