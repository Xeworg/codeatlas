import React from 'react';
import { useService1 } from '../services/Service6.ts';
import { helper6 } from '../utils/helper.ts';

interface Props { id: string; label: string; }

export const Component126 = ({ id, label }: Props) => {
  const svc = useService1();
  return <div id={id}>{label}</div>;
};
