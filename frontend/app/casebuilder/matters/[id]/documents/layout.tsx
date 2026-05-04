export default function CasebuilderDocumentsLayout({
  children,
  modal,
}: LayoutProps<"/casebuilder/matters/[id]/documents">) {
  return (
    <>
      {children}
      {modal}
    </>
  )
}
