import ansiEscapes from "ansi-escapes";
import chalk from "chalk";
import figures from "figures";
import {
  createPrompt,
  isBackspaceKey,
  isDownKey,
  isEnterKey,
  isNumberKey,
  isUpKey,
  makeTheme,
  Separator,
  useKeypress,
  useMemo,
  usePrefix,
  useRef,
  useState,
  ValidationError,
  type Theme,
} from "@inquirer/core";
import type { PartialDeep } from "@inquirer/type";

type SelectTheme = {
  icon: { cursor: string };
  style: { disabled: (text: string) => string };
};

const selectTheme: SelectTheme = {
  icon: { cursor: figures.pointer },
  style: { disabled: (text: string) => chalk.dim(`- ${text}`) },
};

type Action<ActionValue> = {
  value: ActionValue;
  name: string;
  key: string;
};

type Choice<Value> = {
  value: Value;
  name?: string;
  description?: string;
  disabled?: boolean | string;
  type?: never;
};

type ActionSelectConfig<ActionValue, Value> = {
  message: string;
  actions: ReadonlyArray<Action<ActionValue>>;
  choices: ReadonlyArray<Choice<Value> | Separator>;
  helpText?: string;
  pageSize?: number;
  loop?: boolean;
  default?: unknown;
  theme?: PartialDeep<Theme<SelectTheme>>;
};

type ActionSelectResult<ActionValue, Value> = {
  action?: ActionValue;
  answer: Value;
};

type Item<Value> = Separator | Choice<Value>;

function isSelectable<Value>(item: Item<Value>): item is Choice<Value> {
  return !Separator.isSeparator(item) && !item.disabled;
}

function matchesActionKey(
  actionKey: string,
  key: { name?: string; sequence?: string; shift?: boolean },
): boolean {
  if (actionKey === "space") {
    return key.name === "space" || key.sequence === " ";
  }
  if (actionKey.length === 1 && actionKey >= "A" && actionKey <= "Z") {
    return key.name === actionKey.toLowerCase();
  }
  return key.name === actionKey;
}

function formatActionKeyLabel(actionKey: string): string {
  if (actionKey === "space") return "Space";
  if (actionKey === "delete") return "Delete";
  if (actionKey.length === 1) return actionKey;
  return actionKey;
}

function readlineWidth(): number {
  const fromStdout = process.stdout.columns;
  if (typeof fromStdout === "number" && fromStdout > 0) return fromStdout;

  const fromStderr = process.stderr.columns;
  if (typeof fromStderr === "number" && fromStderr > 0) return fromStderr;

  return 80;
}

function breakLines(content: string, width: number): string {
  if (width <= 0) return content;

  const lines: string[] = [];
  let current = "";
  let visible = 0;
  let index = 0;

  const pushCurrent = () => {
    lines.push(current);
    current = "";
    visible = 0;
  };

  while (index < content.length) {
    const char = content[index]!;

    if (char === "\n") {
      pushCurrent();
      index += 1;
      continue;
    }
    if (char === "\r") {
      index += 1;
      continue;
    }

    if (char === "\u001b" && content[index + 1] === "[") {
      let end = index + 2;
      while (end < content.length) {
        const code = content.charCodeAt(end)!;
        if (code >= 0x40 && code <= 0x7e) {
          end += 1;
          break;
        }
        end += 1;
      }
      current += content.slice(index, end);
      index = end;
      continue;
    }

    current += char;
    visible += 1;
    index += 1;

    if (visible >= width) {
      pushCurrent();
    }
  }

  lines.push(current);
  return lines.join("\n");
}

export default createPrompt(
  <ActionValue, Value>(
    config: ActionSelectConfig<ActionValue, Value>,
    done: (result: ActionSelectResult<ActionValue, Value>) => void,
  ): string => {
    const { choices: items, loop = true, pageSize = 7 } = config;
    const theme = makeTheme<SelectTheme>(selectTheme, config.theme);
    const prefix = usePrefix({ theme });
    const [status, setStatus] = useState<"pending" | "done">("pending");
    const searchTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const scrollTopRef = useRef(0);

    const bounds = useMemo(() => {
      const first = items.findIndex(isSelectable);
      let last = -1;
      for (let index = items.length - 1; index >= 0; index -= 1) {
        if (isSelectable(items[index]!)) {
          last = index;
          break;
        }
      }
      if (first < 0) {
        throw new ValidationError(
          "[select prompt] No selectable choices. All choices are disabled.",
        );
      }
      return { first, last };
    }, [items]);

    const defaultItemIndex = useMemo(() => {
      if (!("default" in config)) return -1;
      return items.findIndex(
        (item) => isSelectable(item) && item.value === config.default,
      );
    }, [config.default, items]);

    const [active, setActive] = useState(
      defaultItemIndex === -1 ? bounds.first : defaultItemIndex,
    );
    const [selectedAction, setSelectedAction] = useState<Action<ActionValue> | undefined>(
      undefined,
    );

    const selectedChoice = items[active] as Choice<Value>;

    useKeypress((key, rl) => {
      clearTimeout(searchTimeoutRef.current);

      if (key.ctrl && key.name === "c") {
        process.stdout.write("\n");
        process.exit(130);
      }

      const action = config.actions.find((candidate) =>
        matchesActionKey(candidate.key, key),
      );
      if (action) {
        setStatus("done");
        setSelectedAction(action);
        done({
          action: action.value,
          answer: selectedChoice.value,
        });
        return;
      }

      if (isEnterKey(key)) {
        setStatus("done");
        done({
          action: undefined,
          answer: selectedChoice.value,
        });
        return;
      }

      if (isUpKey(key) || isDownKey(key)) {
        rl.clearLine(0);
        if (
          loop ||
          (isUpKey(key) && active !== bounds.first) ||
          (isDownKey(key) && active !== bounds.last)
        ) {
          const offset = isUpKey(key) ? -1 : 1;
          let next = active;
          do {
            next = (next + offset + items.length) % items.length;
          } while (!isSelectable(items[next]!));
          setActive(next);
        }
        return;
      }

      if (isNumberKey(key)) {
        rl.clearLine(0);
        const position = Number(key.name) - 1;
        const item = items[position];
        if (item != null && isSelectable(item)) {
          setActive(position);
        }
        return;
      }

      if (isBackspaceKey(key)) {
        rl.clearLine(0);
        return;
      }

      const searchTerm = rl.line.toLowerCase();
      const matchIndex = items.findIndex((item) => {
        if (Separator.isSeparator(item) || !isSelectable(item)) return false;
        return String(item.name || item.value)
          .toLowerCase()
          .startsWith(searchTerm);
      });

      if (matchIndex >= 0) {
        setActive(matchIndex);
      }

      searchTimeoutRef.current = setTimeout(() => {
        rl.clearLine(0);
      }, 700);
    });

    const message = theme.style.message(config.message, status);
    const helpTip = config.helpText
      ? config.helpText
      : config.actions
          .map(
            (action) =>
              `${theme.style.help(action.name)} ${theme.style.key(formatActionKeyLabel(action.key))}`,
          )
          .join(" ");

    const width = readlineWidth();
    const renderedItems = items.map((item, index) =>
      breakLines(
        (() => {
          if (Separator.isSeparator(item)) {
            return ` ${item.separator}`;
          }

          const line = item.name || item.value;
          if (item.disabled) {
            const disabledLabel =
              typeof item.disabled === "string" ? item.disabled : "(disabled)";
            return theme.style.disabled(`${line} ${disabledLabel}`);
          }

          const color = index === active ? theme.style.highlight : (x: string) => x;
          const cursor = index === active ? theme.icon.cursor : " ";
          return color(`${cursor} ${line}`);
        })(),
        width,
      ).split("\n"),
    );

    const flattened = renderedItems.flat();
    const renderedLength = flattened.length;
    const activeStart = renderedItems
      .slice(0, active)
      .reduce((acc, lines) => acc + lines.length, 0);
    const activeHeight = Math.max(1, renderedItems[active]?.length ?? 1);
    const activeEnd = activeStart + activeHeight;
    const maxTop = Math.max(0, renderedLength - pageSize);
    let top = scrollTopRef.current;

    if (renderedLength <= pageSize) {
      top = 0;
    } else {
      if (activeStart < top) {
        top = activeStart;
      } else if (activeEnd > top + pageSize) {
        top = activeEnd - pageSize;
      }
      top = Math.max(0, Math.min(top, maxTop));
    }
    scrollTopRef.current = top;

    const pageLines = flattened.slice(top, top + pageSize).join("\n");

    if (status === "done") {
      const answer = selectedChoice.name || String(selectedChoice.value);
      if (selectedAction) {
        const action = selectedAction.name || String(selectedAction.value);
        const hideSelectedAnswerKeys = new Set(["a", "b", "c", "n", "r", "u", "space"]);
        if (hideSelectedAnswerKeys.has(selectedAction.key)) {
          return `${prefix} ${message} ${theme.style.help(action)}`;
        }
        return `${prefix} ${message} ${theme.style.help(action)} ${theme.style.answer(answer)}`;
      }
      return `${prefix} ${message} ${theme.style.answer(answer)}`;
    }

    const choiceDescription = selectedChoice.description
      ? `\n${selectedChoice.description}`
      : "";

    return `${[prefix, message, helpTip].filter(Boolean).join(" ")}\n${pageLines}${choiceDescription}${ansiEscapes.cursorHide}`;
  },
);

export { Separator };
