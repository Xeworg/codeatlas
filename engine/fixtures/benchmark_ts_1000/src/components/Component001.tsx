import React from 'react';
import { useService1 } from '../services/Service1.ts';
import { helper1 } from '../utils/helper.ts';

interface Props { id: string; label: string; }

export const Component001 = ({ id, label }: Props) => {
  const svc = useService1();
  return <div id={id}>{label}</div>;
};
