import React from "react";

export interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {}

export function Textarea({ className = "", ...rest }: TextareaProps) {
  return <textarea className={`ui-textarea ${className}`.trim()} {...rest} />;
}
