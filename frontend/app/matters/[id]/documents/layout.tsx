export default function DocumentsLayout({
  children,
  modal,
}: LayoutProps<"/matters/[id]/documents">) {
  return (
    <>
      {children}
      {modal}
    </>
  )
}
