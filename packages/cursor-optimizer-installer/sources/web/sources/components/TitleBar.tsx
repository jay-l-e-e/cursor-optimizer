import { TitleBar as SharedTitleBar } from "@cursor-optimizer/user-interface";

export default function TitleBar(props: { title: string }) {
  return <SharedTitleBar title={props.title} />;
}
